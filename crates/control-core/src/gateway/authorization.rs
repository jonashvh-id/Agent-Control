use crate::{
    App, agents,
    connector::Tool,
    error::{Error, Result},
    lifecycle,
    models::{Agent, Policy},
    policy,
};
pub async fn check(
    app: &App,
    db: &mut sqlx::SqliteConnection,
    token: &str,
    connector: &str,
    tool: &Tool,
) -> Result<(Agent, Policy)> {
    let actor = agents::resolve(app, db, token).await?;
    let mode = for_actor(app, db, &actor, connector, tool).await?;
    Ok((actor, mode))
}
// The caller must have resolved a current actor. Also used by human-only access diagnostics.
pub async fn for_actor(
    app: &App,
    db: &mut sqlx::SqliteConnection,
    actor: &Agent,
    connector: &str,
    tool: &Tool,
) -> Result<Policy> {
    lifecycle::check(db, connector).await?;
    let mode = policy::current(db, actor, connector, tool).await?;
    if mode == Policy::Deny {
        return Err(Error::new("policy_denied", 403));
    }
    if !app.config.execution_enabled {
        return Err(Error::new("execution_disabled", 503));
    }
    let (provider, _) = app.tool(connector, &tool.name)?;
    if provider.requires_resources() {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT state,verification FROM connection_secrets WHERE connector=?")
                .bind(connector)
                .fetch_optional(&mut *db)
                .await?;
        if !row.is_some_and(|(state, v)| {
            state == "connected"
                && serde_json::from_str::<serde_json::Value>(&v)
                    .is_ok_and(|v| v["verified"] == true)
        }) {
            return Err(Error::new("connection_attention_required", 403));
        }
        let resources = crate::resources::available(db, &actor.id, connector).await?;
        if resources.is_empty() {
            return Err(Error::new("resource_permission_denied", 403));
        }
        if mode == Policy::Allow && resources.iter().all(|r| r["mode"] == "ask") {
            return Ok(Policy::Ask);
        }
    }
    Ok(mode)
}
