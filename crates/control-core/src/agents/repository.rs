use crate::{
    error::{Error, Result},
    persistence::{now, row_json},
};
use serde_json::{Value, json};
use sqlx::{SqliteConnection, SqlitePool, sqlite::SqliteRow};
pub async fn get(db: &mut SqliteConnection, id: &str) -> Result<Value> {
    sqlx::query("SELECT * FROM agents WHERE id=?")
        .bind(id)
        .fetch_optional(db)
        .await?
        .map(|r| row_json(&r))
        .ok_or(Error::new("agent_not_found", 404))
}
pub async fn list(pool: &SqlitePool) -> Result<Value> {
    Ok(Value::Array(
        sqlx::query("SELECT * FROM agents ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?
            .iter()
            .map(row_json)
            .collect(),
    ))
}
pub async fn insert(
    db: &mut SqliteConnection,
    id: &str,
    human: &str,
    input: &super::Create,
) -> Result<()> {
    let now = now();
    sqlx::query("INSERT INTO agents(id,name,description,provider,runtime,scopes,created_by,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)")
        .bind(id).bind(&input.name).bind(&input.description).bind(&input.provider).bind(&input.runtime).bind(json!(input.scopes).to_string()).bind(human).bind(&now).bind(&now).execute(db).await?;
    Ok(())
}
pub async fn issue(
    db: &mut SqliteConnection,
    id: &str,
    agent: &str,
    hash: &str,
    scopes: &[String],
) -> Result<()> {
    sqlx::query("INSERT INTO credentials VALUES(?,?,?,?,?,?,NULL)")
        .bind(id)
        .bind(agent)
        .bind(hash)
        .bind(json!(scopes).to_string())
        .bind("active")
        .bind(now())
        .execute(db)
        .await?;
    Ok(())
}
pub async fn credential(
    db: &mut SqliteConnection,
    hash: &str,
    active: bool,
) -> Result<Option<SqliteRow>> {
    Ok(sqlx::query(if active {
        "SELECT id,agent_id,scopes FROM credentials WHERE token_hash=? AND status='active'"
    } else {
        "SELECT id,agent_id,scopes FROM credentials WHERE token_hash=?"
    })
    .bind(hash)
    .fetch_optional(db)
    .await?)
}
pub async fn revoke(db: &mut SqliteConnection, agent: &str, credential: &str) -> Result<u64> {
    Ok(sqlx::query("UPDATE credentials SET status='revoked',revoked_at=? WHERE id=? AND agent_id=? AND status='active'").bind(now()).bind(credential).bind(agent).execute(db).await?.rows_affected())
}
pub async fn credentials(db: &mut SqliteConnection, agent: &str) -> Result<Value> {
    Ok(Value::Array(sqlx::query("SELECT id,agent_id,scopes,status,created_at,revoked_at FROM credentials WHERE agent_id=? ORDER BY created_at DESC").bind(agent).fetch_all(db).await?.iter().map(row_json).collect()))
}
pub async fn set_scopes(db: &mut SqliteConnection, agent: &str, scopes: &[String]) -> Result<()> {
    sqlx::query("UPDATE agents SET scopes=?,updated_at=? WHERE id=?")
        .bind(json!(scopes).to_string())
        .bind(now())
        .bind(agent)
        .execute(db)
        .await?;
    Ok(())
}
