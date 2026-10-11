use crate::{
    error::Result,
    persistence::{now, row_json},
};
use serde_json::Value;
use sqlx::{SqliteConnection, SqlitePool};
pub async fn mode(
    db: &mut SqliteConnection,
    agent: &str,
    connector: &str,
    tool: &str,
) -> Result<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT mode FROM policies WHERE agent_id=? AND connector=? AND tool=?")
            .bind(agent)
            .bind(connector)
            .bind(tool)
            .fetch_optional(db)
            .await?,
    )
}
pub async fn set(db: &mut SqliteConnection, actor: &str, input: &super::Set) -> Result<()> {
    sqlx::query("INSERT INTO policies VALUES(?,?,?,?,?,?) ON CONFLICT(agent_id,connector,tool) DO UPDATE SET mode=excluded.mode,updated_by=excluded.updated_by,updated_at=excluded.updated_at").bind(&input.agent_id).bind(&input.connector).bind(&input.tool).bind(input.mode.as_str()).bind(actor).bind(now()).execute(db).await?;
    Ok(())
}
pub async fn list(pool: &SqlitePool) -> Result<Value> {
    Ok(Value::Array(
        sqlx::query("SELECT * FROM policies ORDER BY agent_id,connector,tool")
            .fetch_all(pool)
            .await?
            .iter()
            .map(row_json)
            .collect(),
    ))
}
