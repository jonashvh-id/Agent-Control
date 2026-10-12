//! Authorized operator projections. Stored audit remains payload-free.
use crate::{App, error::Result, persistence::row_json};
use serde_json::{Value, json};
pub async fn activity(app: &App) -> Result<Value> {
    let mut events = crate::audit::list(&app.db.pool).await?;
    for event in events.as_array_mut().unwrap() {
        let resource = event["resource_ref"].as_str().unwrap_or("");
        if !resource.is_empty()
            && let Some(cipher) = sqlx::query_scalar::<_, String>(
                "SELECT metadata_ciphertext FROM resources WHERE id=?",
            )
            .bind(resource)
            .fetch_optional(&app.db.pool)
            .await?
        {
            let data = app
                .db
                .keys
                .open_private(&format!("resource:{resource}"), &cipher)?;
            event["resource_name"] = data["name"].clone();
        }
        let mut objects = vec![];
        for reference in event["object_refs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if let Some(cipher) = sqlx::query_scalar::<_, String>(
                "SELECT metadata_ciphertext FROM resource_objects WHERE ref=?",
            )
            .bind(reference)
            .fetch_optional(&app.db.pool)
            .await?
            {
                objects.push(
                    app.db
                        .keys
                        .open_private(&format!("object:{reference}"), &cipher)?,
                );
            }
        }
        event["objects"] = json!(objects);
    }
    Ok(events)
}
pub async fn dashboard(app: &App) -> Result<Value> {
    let rows=sqlx::query("SELECT a.*,(SELECT COUNT(*) FROM credentials c WHERE c.agent_id=a.id AND c.status='active') AS active_credentials,(SELECT MAX(e.created_at) FROM audit_events e JOIN credentials c ON c.id=e.actor_id AND c.status='active' WHERE e.agent_id=a.id AND e.actor_kind='agent' AND e.action NOT IN ('mcp.authentication','gateway.authentication')) AS last_request_at,(SELECT COUNT(*) FROM audit_events e WHERE e.agent_id=a.id AND e.decision='deny' AND julianday(e.created_at)>=julianday('now','-1 day')) AS denied_attempts_24h FROM agents a ORDER BY a.created_at DESC").fetch_all(&app.db.pool).await?;
    let agents: Vec<Value> = rows.iter().map(row_json).collect();
    let completed:i64=sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE action='execution.executed' AND julianday(created_at)>=julianday('now','-1 day')").fetch_one(&app.db.pool).await?;
    let denied:i64=sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE decision='deny' AND (actor_kind='agent' OR action IN ('mcp.authentication','gateway.authentication')) AND julianday(created_at)>=julianday('now','-1 day')").fetch_one(&app.db.pool).await?;
    let uncertain: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM executions WHERE status='uncertain'")
            .fetch_one(&app.db.pool)
            .await?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM approvals WHERE status='pending' AND expires_at>?",
    )
    .bind(crate::persistence::epoch())
    .fetch_one(&app.db.pool)
    .await?;
    let mut connections = vec![];
    for provider in &app.connectors {
        let enabled: i64 = sqlx::query_scalar("SELECT enabled FROM connector_state WHERE id=?")
            .bind(provider.id())
            .fetch_one(&app.db.pool)
            .await?;
        let status = if let Some(control) = provider.management() {
            control.status().await?
        } else {
            json!({"mode":"manual_or_code_configured","state":"native_credential_not_verified","automatic_refresh":false})
        };
        connections
            .push(json!({"connector":provider.id(),"enabled":enabled==1,"connection":status}));
    }
    let policies=sqlx::query("SELECT p.agent_id,p.resource_id,p.mode FROM resource_policies p JOIN resources r ON r.id=p.resource_id ORDER BY p.updated_at DESC").fetch_all(&app.db.pool).await?.iter().map(row_json).collect::<Vec<_>>();
    let store = crate::resources::Store {
        pool: app.db.pool.clone(),
        keys: app.db.keys.clone(),
    };
    let mut resources = vec![];
    for provider in &app.connectors {
        resources.extend(store.list(provider.id()).await?);
    }
    Ok(
        json!({"agents":agents,"metrics":{"completed_24h":completed,"denied_attempts_24h":denied,"pending_approvals":pending,"uncertain_executions":uncertain},"connections":connections,"resources":resources,"resource_policies":policies,"activity":activity(app).await?,"presence":"Stateless MCP has no persistent online presence. Last request and active credentials are separate observations."}),
    )
}
