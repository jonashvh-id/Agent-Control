use crate::{
    audit,
    error::{Error, Result},
    models::{Agent, Proposal},
    persistence::{epoch, now},
};
use sqlx::{SqliteConnection, sqlite::SqliteRow};
pub async fn expire(db: &mut SqliteConnection) -> Result<()> {
    sqlx::query("UPDATE approvals SET status='expired' WHERE status IN ('pending','approved') AND expires_at<=?").bind(epoch()).execute(db).await?;
    Ok(())
}
pub async fn insert(
    db: &mut SqliteConnection,
    id: &str,
    agent: &Agent,
    p: &Proposal,
    hash: &str,
    encrypted: &str,
    seconds: u64,
) -> Result<()> {
    sqlx::query("INSERT INTO approvals(id,agent_id,credential_id,connector,tool,input_hash,idempotency_key,input_summary,status,expires_at,created_at,preview_ciphertext) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(id).bind(&agent.id).bind(&agent.credential_id).bind(&p.connector).bind(&p.tool).bind(hash).bind(&p.idempotency_key).bind(audit::summarize(&serde_json::json!(p.parameters)).to_string()).bind("pending").bind(epoch()+seconds as f64).bind(now()).bind(encrypted).execute(db).await?;
    Ok(())
}
pub async fn get(db: &mut SqliteConnection, id: &str) -> Result<SqliteRow> {
    sqlx::query("SELECT * FROM approvals WHERE id=?")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(Error::new("approval_not_found", 404))
}
pub async fn pending_list(db: &mut SqliteConnection) -> Result<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM approvals ORDER BY created_at DESC LIMIT 100")
            .fetch_all(db)
            .await?,
    )
}
pub async fn decide(
    db: &mut SqliteConnection,
    id: &str,
    actor: &str,
    decision: &str,
) -> Result<u64> {
    Ok(sqlx::query("UPDATE approvals SET status=?,decided_by=?,decided_at=? WHERE id=? AND status='pending' AND expires_at>?").bind(decision).bind(actor).bind(now()).bind(id).bind(epoch()).execute(db).await?.rows_affected())
}
pub async fn approver_enabled(
    db: &mut SqliteConnection,
    human: Option<String>,
) -> Result<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT enabled FROM humans WHERE id=?")
        .bind(human)
        .fetch_optional(db)
        .await?)
}
pub async fn consume(db: &mut SqliteConnection, id: &str) -> Result<u64> {
    Ok(sqlx::query(
        "UPDATE approvals SET status='consumed' WHERE id=? AND status='approved' AND expires_at>?",
    )
    .bind(id)
    .bind(epoch())
    .execute(db)
    .await?
    .rows_affected())
}
pub async fn authority(db: &mut SqliteConnection, id: &str) -> Result<SqliteRow> {
    sqlx::query("SELECT a.status,a.expires_at,h.enabled FROM approvals a LEFT JOIN humans h ON h.id=a.decided_by WHERE a.id=?").bind(id).fetch_optional(db).await?.ok_or(Error::new("approval_not_found",404))
}
