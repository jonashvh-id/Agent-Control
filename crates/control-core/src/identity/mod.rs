pub mod repository;
use crate::{
    App,
    audit::{self, Event},
    error::{Error, Result},
    models::Human,
    persistence::{epoch, id},
    security::{keys::random_token, passwords},
};
use sqlx::{Row, SqliteConnection};
pub async fn create_admin(app: &App, username: &str, password: &str, role: &str) -> Result<String> {
    if !regex::Regex::new(r"^[a-z0-9_.@-]{1,80}$")
        .unwrap()
        .is_match(username)
        || !matches!(role, "admin" | "reviewer")
    {
        return Err(Error::new("invalid_request", 422));
    }
    let hash = passwords::hash(password)?;
    let id = id();
    let mut db = app.db.write().await?;
    repository::insert(&mut db, &id, username, &hash, role).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &id,
            action: "administrator.bootstrap",
            decision: "allow",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(id)
}
pub async fn login(app: &App, username: &str, password: &str, ip: &str) -> Result<(String, Human)> {
    let username = username.trim().to_lowercase();
    let key = app.db.keys.hash(&format!("{username}|{ip}"));
    // Reserve a rate-limit slot durably before expensive password verification.
    let mut db = app.db.write().await?;
    repository::reserve_attempt(&mut db, &key).await?;
    db.commit().await?;
    let mut connection = app.db.pool.acquire().await?;
    let row = repository::lookup(&mut connection, &username).await?;
    drop(connection);
    let hash = match &row {
        Some(r) => r.get::<String, _>("password_hash"),
        None => app.dummy_hash.clone(),
    };
    let verified_hash = hash.clone();
    let password_permit = app
        .password_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::new("authentication_busy", 429))?;
    let pw = zeroize::Zeroizing::new(password.to_string());
    let valid = tokio::task::spawn_blocking(move || {
        let _permit = password_permit;
        passwords::verify(&pw, &hash)
    })
    .await
    .map_err(|_| Error::new("authentication_failed", 503))?;
    let mut db = app.db.write().await?;
    let row = if valid {
        repository::verified_human(&mut db, &username, &verified_hash).await?
    } else {
        None
    };
    let Some(row) = row else {
        audit::record(
            &mut db,
            Event {
                actor_kind: "unknown",
                action: "login",
                decision: "denied",
                ..Default::default()
            },
        )
        .await?;
        db.commit().await?;
        return Err(Error::new("invalid_credentials", 401));
    };
    let token = random_token();
    let actor = Human {
        id: row.get("id"),
        username: row.get("username"),
        role: row.get("role"),
        csrf: random_token(),
    };
    repository::issue(
        &mut db,
        &app.db.keys.hash(&token),
        &actor.id,
        &actor.csrf,
        epoch() + app.config.session_seconds as f64,
        &key,
    )
    .await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            action: "login",
            decision: "allow",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok((token, actor))
}
pub async fn authenticate(app: &App, db: &mut SqliteConnection, token: &str) -> Result<Human> {
    let r = repository::session(db, &app.db.keys.hash(token)).await?;
    Ok(Human {
        id: r.get("id"),
        username: r.get("username"),
        role: r.get("role"),
        csrf: r.get("csrf"),
    })
}
pub async fn logout(app: &App, token: &str, actor: &Human) -> Result<()> {
    let mut db = app.db.write().await?;
    repository::revoke_session(&mut db, &app.db.keys.hash(token)).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            action: "logout",
            decision: "allow",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
