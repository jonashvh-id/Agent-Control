pub mod repository;
use crate::{
    App,
    audit::{self, Event},
    error::{Error, Result},
    models::Human,
};
use sqlx::SqliteConnection;
pub async fn check(db: &mut SqliteConnection, connector: &str) -> Result<()> {
    let stopped = repository::stopped(db).await?;
    if stopped != 0 {
        return Err(Error::new("emergency_stop", 403));
    }
    let enabled = repository::enabled(db, connector).await?;
    if enabled != Some(1) {
        return Err(Error::new("connector_disabled", 403));
    }
    Ok(())
}
pub async fn stop(app: &App, actor: &Human, stopped: bool) -> Result<()> {
    let mut db = app.db.write().await?;
    repository::stop(&mut db, stopped).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            action: "emergency_stop",
            decision: if stopped { "stopped" } else { "resumed" },
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
pub async fn connector(app: &App, actor: &Human, name: &str, enabled: bool) -> Result<()> {
    let mut db = app.db.write().await?;
    let n = repository::connector(&mut db, name, enabled).await?;
    if n != 1 {
        return Err(Error::new("connector_not_found", 404));
    }
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            connector: name,
            action: "connector.status",
            decision: if enabled { "enabled" } else { "disabled" },
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
pub async fn agent(app: &App, actor: &Human, id: &str, status: &str) -> Result<()> {
    if !matches!(status, "active" | "paused" | "disabled") {
        return Err(Error::new("invalid_request", 422));
    }
    let mut db = app.db.write().await?;
    let n = repository::agent(&mut db, id, status).await?;
    if n != 1 {
        return Err(Error::new("agent_not_found", 404));
    }
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id: id,
            action: "agent.status",
            decision: status,
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
