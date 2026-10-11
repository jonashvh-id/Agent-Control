use crate::{
    App,
    audit::{self, Event},
    error::{Error, Result},
    models::{Agent, Human, Proposal},
    persistence::{epoch, id, row_json},
};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
pub mod repository;
pub use repository::expire;
pub async fn create(
    app: &App,
    db: &mut SqliteConnection,
    agent: &Agent,
    p: &Proposal,
    hash: &str,
    preview: Option<Value>,
) -> Result<String> {
    let id = id();
    let encrypted = preview
        .map(|v| app.db.keys.seal(&v))
        .transpose()?
        .unwrap_or_default();
    repository::insert(
        db,
        &id,
        agent,
        p,
        hash,
        &encrypted,
        app.config.approval_seconds,
    )
    .await?;
    Ok(id)
}
pub async fn get(app: &App, db: &mut SqliteConnection, id: &str) -> Result<Value> {
    let row = repository::get(db, id).await?;
    let mut value = row_json(&row);
    let cipher: String = row.get("preview_ciphertext");
    value.as_object_mut().unwrap().remove("preview_ciphertext");
    value["input_preview"] = if cipher.is_empty() {
        Value::Null
    } else {
        app.db.keys.open(&cipher)?
    };
    Ok(value)
}
pub async fn list(app: &App) -> Result<Value> {
    let mut db = app.db.write().await?;
    expire(&mut db).await?;
    let ids = repository::pending_list(&mut db).await?;
    let mut list = vec![];
    for id in ids {
        list.push(get(app, &mut db, &id).await?);
    }
    db.commit().await?;
    Ok(json!(list))
}
pub async fn decide(app: &App, actor: &Human, id: &str, decision: &str) -> Result<Value> {
    if !matches!(decision, "approved" | "denied") {
        return Err(Error::new("invalid_request", 422));
    }
    let mut db = app.db.write().await?;
    expire(&mut db).await?;
    let n = repository::decide(&mut db, id, &actor.id, decision).await?;
    if n != 1 {
        get(app, &mut db, id).await?;
        db.commit().await?;
        return Err(Error::new("approval_not_pending", 409));
    }
    let value = get(app, &mut db, id).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id: value["agent_id"].as_str().unwrap_or_default(),
            connector: value["connector"].as_str().unwrap_or_default(),
            tool: value["tool"].as_str().unwrap_or_default(),
            action: "approval.decide",
            decision,
            approval_id: id,
            input_hash: value["input_hash"].as_str().unwrap_or_default(),
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(value)
}
pub async fn consume(
    db: &mut SqliteConnection,
    id: &str,
    agent: &Agent,
    p: &Proposal,
    hash: &str,
) -> Result<()> {
    let row = repository::get(db, id).await?;
    let enabled =
        repository::approver_enabled(db, row.get::<Option<String>, _>("decided_by")).await?;
    if enabled != Some(1) {
        return Err(Error::new("approval_authority_expired", 403));
    }
    for (key, value) in [
        ("agent_id", &agent.id),
        ("credential_id", &agent.credential_id),
        ("connector", &p.connector),
        ("tool", &p.tool),
        ("input_hash", &hash.to_string()),
        ("idempotency_key", &p.idempotency_key),
    ] {
        if row.get::<String, _>(key) != *value {
            return Err(Error::new("approval_binding_mismatch", 403));
        }
    }
    let n = repository::consume(db, id).await?;
    if n != 1 {
        return Err(Error::new("approval_unusable", 409));
    }
    Ok(())
}
pub async fn revalidate(db: &mut SqliteConnection, id: &str) -> Result<()> {
    let row = repository::authority(db, id).await?;
    if row.get::<String, _>("status") != "consumed"
        || row.get::<f64, _>("expires_at") <= epoch()
        || row.get::<Option<i64>, _>("enabled") != Some(1)
    {
        return Err(Error::new("approval_authority_expired", 403));
    }
    Ok(())
}
