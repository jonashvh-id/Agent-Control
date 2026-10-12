use super::{Event, summarize};
use crate::{
    error::Result,
    persistence::{id, now, row_json},
};
use serde_json::{Value, json};
use sqlx::SqliteConnection;
pub async fn record(db: &mut SqliteConnection, event: Event<'_>) -> Result<()> {
    sqlx::query("INSERT INTO audit_events(id,created_at,actor_kind,actor_id,agent_id,connector,tool,action,decision,approval_id,execution_id,correlation_id,input_hash,input_summary,result_summary) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(id()).bind(now()).bind(event.actor_kind).bind(event.actor_id).bind(event.agent_id).bind(event.connector).bind(event.tool)
        .bind(event.action).bind(event.decision).bind(event.approval_id).bind(event.execution_id).bind(if event.correlation_id.is_empty(){id()}else{event.correlation_id.into()})
        .bind(event.input_hash).bind(event.parameters.map(summarize).unwrap_or(json!({})).to_string())
        .bind(event.receipt.cloned().unwrap_or(json!({})).to_string()).execute(db).await?;
    Ok(())
}
pub async fn list(pool: &sqlx::SqlitePool) -> Result<Value> {
    Ok(Value::Array(
        sqlx::query("SELECT * FROM audit_events ORDER BY sequence DESC LIMIT 100")
            .fetch_all(pool)
            .await?
            .iter()
            .map(row_json)
            .collect(),
    ))
}
