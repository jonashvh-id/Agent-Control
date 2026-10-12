use crate::{error::Result, persistence::now};
use sqlx::SqliteConnection;
pub async fn stopped(db: &mut SqliteConnection) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT stopped FROM emergency_state WHERE id=1")
            .fetch_one(db)
            .await?,
    )
}
pub async fn enabled(db: &mut SqliteConnection, connector: &str) -> Result<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT enabled FROM connector_state WHERE id=?")
            .bind(connector)
            .fetch_optional(db)
            .await?,
    )
}
pub async fn stop(db: &mut SqliteConnection, stopped: bool) -> Result<()> {
    sqlx::query("UPDATE emergency_state SET stopped=? WHERE id=1")
        .bind(stopped)
        .execute(db)
        .await?;
    Ok(())
}
pub async fn connector(db: &mut SqliteConnection, name: &str, enabled: bool) -> Result<u64> {
    Ok(
        sqlx::query("UPDATE connector_state SET enabled=? WHERE id=?")
            .bind(enabled)
            .bind(name)
            .execute(db)
            .await?
            .rows_affected(),
    )
}
pub async fn agent(db: &mut SqliteConnection, id: &str, status: &str) -> Result<u64> {
    Ok(
        sqlx::query("UPDATE agents SET status=?,updated_at=? WHERE id=?")
            .bind(status)
            .bind(now())
            .bind(id)
            .execute(db)
            .await?
            .rows_affected(),
    )
}
