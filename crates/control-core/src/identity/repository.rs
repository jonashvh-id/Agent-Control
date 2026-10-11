use crate::{
    error::{Error, Result},
    persistence::epoch,
};
use sqlx::{SqliteConnection, sqlite::SqliteRow};
pub async fn insert(
    db: &mut SqliteConnection,
    id: &str,
    username: &str,
    hash: &str,
    role: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO humans(id,username,password_hash,role) VALUES(?,?,?,?)")
        .bind(id)
        .bind(username)
        .bind(hash)
        .bind(role)
        .execute(db)
        .await
        .map_err(|_| Error::new("username_exists", 409))?;
    Ok(())
}
pub async fn reserve_attempt(db: &mut SqliteConnection, key: &str) -> Result<()> {
    sqlx::query("DELETE FROM login_attempts WHERE reset_at<=?")
        .bind(epoch())
        .execute(&mut *db)
        .await?;
    let attempts: Option<i64> =
        sqlx::query_scalar("SELECT attempts FROM login_attempts WHERE key=?")
            .bind(key)
            .fetch_optional(&mut *db)
            .await?;
    if attempts.unwrap_or(0) >= 5 {
        return Err(Error::new("login_rate_limited", 429));
    }
    sqlx::query("INSERT INTO login_attempts VALUES(?,1,?) ON CONFLICT(key) DO UPDATE SET attempts=attempts+1").bind(key).bind(epoch()+300.).execute(db).await?;
    Ok(())
}
pub async fn lookup(db: &mut SqliteConnection, username: &str) -> Result<Option<SqliteRow>> {
    Ok(sqlx::query("SELECT * FROM humans WHERE username=?")
        .bind(username)
        .fetch_optional(db)
        .await?)
}
pub async fn verified_human(
    db: &mut SqliteConnection,
    username: &str,
    hash: &str,
) -> Result<Option<SqliteRow>> {
    Ok(sqlx::query(
        "SELECT id,username,role FROM humans WHERE username=? AND enabled=1 AND password_hash=?",
    )
    .bind(username)
    .bind(hash)
    .fetch_optional(db)
    .await?)
}
pub async fn issue(
    db: &mut SqliteConnection,
    hash: &str,
    human: &str,
    csrf: &str,
    expires: f64,
    attempt: &str,
) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE expires_at<=?")
        .bind(epoch())
        .execute(&mut *db)
        .await?;
    sqlx::query("INSERT INTO sessions VALUES(?,?,?,?)")
        .bind(hash)
        .bind(human)
        .bind(csrf)
        .bind(expires)
        .execute(&mut *db)
        .await?;
    sqlx::query("DELETE FROM login_attempts WHERE key=?")
        .bind(attempt)
        .execute(db)
        .await?;
    Ok(())
}
pub async fn session(db: &mut SqliteConnection, hash: &str) -> Result<SqliteRow> {
    sqlx::query("SELECT h.id,h.username,h.role,s.csrf FROM sessions s JOIN humans h ON h.id=s.human_id WHERE s.token_hash=? AND s.expires_at>? AND h.enabled=1").bind(hash).bind(epoch()).fetch_optional(db).await?.ok_or(Error::new("human_authentication_required",401))
}
pub async fn revoke_session(db: &mut SqliteConnection, hash: &str) -> Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash=?")
        .bind(hash)
        .execute(db)
        .await?;
    Ok(())
}
