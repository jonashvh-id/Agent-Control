//! Durable compare-and-swap transitions; no provider retry decisions are made here.
use crate::{
    error::{Error, Result},
    models::ExecutionState,
    persistence::now,
};
use serde_json::Value;
use sqlx::{Row, SqliteConnection};
pub struct Record {
    pub id: String,
    pub state: ExecutionState,
    pub receipt: Option<Value>,
}
pub async fn previous(
    db: &mut SqliteConnection,
    agent: &str,
    key: &str,
    hash: &str,
) -> Result<Option<Record>> {
    let row = sqlx::query("SELECT * FROM executions WHERE agent_id=? AND idempotency_key=?")
        .bind(agent)
        .bind(key)
        .fetch_optional(db)
        .await?;
    row.map(|r| {
        if r.get::<String, _>("request_hash") != hash {
            return Err(Error::new("idempotency_conflict", 409));
        }
        let state = serde_json::from_value::<ExecutionState>(serde_json::json!(
            r.get::<String, _>("status")
        ))
        .map_err(|_| Error::new("invalid_execution_state", 503))?;
        Ok(Record {
            id: r.get("id"),
            state,
            receipt: r
                .get::<Option<String>, _>("result_receipt")
                .and_then(|s| serde_json::from_str(&s).ok()),
        })
    })
    .transpose()
}
pub async fn claim(
    db: &mut SqliteConnection,
    id: &str,
    agent: &str,
    key: &str,
    hash: &str,
    approval: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO executions(id,agent_id,idempotency_key,request_hash,approval_id,status,created_at) VALUES(?,?,?,?,?,'prepared',?)").bind(id).bind(agent).bind(key).bind(hash).bind(approval).bind(now()).execute(db).await?;
    Ok(())
}
pub async fn dispatch(db: &mut SqliteConnection, id: &str) -> Result<bool> {
    Ok(sqlx::query("UPDATE executions SET status='dispatched',dispatch_started_at=? WHERE id=? AND status='prepared' AND dispatch_started_at IS NULL").bind(now()).bind(id).execute(db).await?.rows_affected()==1)
}
pub async fn finish(
    db: &mut SqliteConnection,
    id: &str,
    state: ExecutionState,
    receipt: Option<&Value>,
) -> Result<()> {
    let expected = match state {
        ExecutionState::Executed => "dispatched",
        ExecutionState::Failed | ExecutionState::Uncertain => "",
        _ => return Err(Error::new("invalid_execution_transition", 409)),
    };
    let n=sqlx::query("UPDATE executions SET status=?,result_receipt=? WHERE id=? AND ((?='dispatched' AND status='dispatched') OR (?='' AND status IN ('prepared','dispatched')))")
        .bind(state.as_str()).bind(receipt.map(Value::to_string)).bind(id).bind(expected).bind(expected).execute(db).await?.rows_affected();
    if n != 1 {
        return Err(Error::new("invalid_execution_transition", 409));
    }
    Ok(())
}
pub async fn recover(pool: &sqlx::SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE executions SET status='uncertain' WHERE status IN ('prepared','dispatched')",
    )
    .execute(pool)
    .await?;
    Ok(())
}
