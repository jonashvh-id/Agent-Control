use crate::{
    App,
    audit::{self, Event},
    error::{Error, Result},
    models::{Agent, Human},
    persistence::id,
    security::keys::random_token,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Create {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub runtime: String,
    #[serde(default)]
    pub scopes: Vec<String>,
}
pub fn scopes_valid(scopes: &[String]) -> bool {
    scopes.len() <= 100
        && scopes.iter().all(|s| {
            regex::Regex::new(r"^[a-z][a-z0-9_.-]{0,63}:[a-z][a-z0-9_-]{0,31}$")
                .unwrap()
                .is_match(s)
        })
}
pub mod repository;
pub use repository::{credentials, get};
pub async fn list(app: &App) -> Result<Value> {
    repository::list(&app.db.pool).await
}
pub async fn create(app: &App, actor: &Human, mut input: Create) -> Result<Value> {
    if input.name.is_empty()
        || input.name.chars().count() > 80
        || input.description.chars().count() > 500
        || input.provider.chars().count() > 40
        || input.runtime.chars().count() > 80
        || !scopes_valid(&input.scopes)
    {
        return Err(Error::new("invalid_request", 422));
    }
    input.scopes.sort();
    input.scopes.dedup();
    let id = id();
    let mut db = app.db.write().await?;
    repository::insert(&mut db, &id, &actor.id, &input).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id: &id,
            action: "agent.create",
            decision: "allow",
            ..Default::default()
        },
    )
    .await?;
    let result = get(&mut db, &id).await?;
    db.commit().await?;
    Ok(result)
}
pub async fn issue(
    app: &App,
    actor: &Human,
    agent_id: &str,
    mut scopes: Vec<String>,
) -> Result<Value> {
    if !scopes_valid(&scopes) {
        return Err(Error::new("invalid_request", 422));
    }
    scopes.sort();
    scopes.dedup();
    let mut db = app.db.write().await?;
    let agent = get(&mut db, agent_id).await?;
    if agent["status"] != "active" {
        return Err(Error::new("agent_inactive", 409));
    }
    if !scopes.iter().all(|s| {
        agent["scopes"]
            .as_array()
            .is_some_and(|a| a.contains(&json!(s)))
    }) {
        return Err(Error::new("credential_scope_escalation", 403));
    }
    let token = format!("ac_agent_{}", random_token());
    let id = id();
    repository::issue(&mut db, &id, agent_id, &app.db.keys.hash(&token), &scopes).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id,
            action: "credential.issue",
            decision: "allow",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(json!({"id":id,"bearer_token":token,"display_once":true}))
}
pub async fn resolve(app: &App, db: &mut SqliteConnection, token: &str) -> Result<Agent> {
    let r = repository::credential(db, &app.db.keys.hash(token), true)
        .await?
        .ok_or(Error::new("agent_authentication_required", 401))?;
    let agent = get(db, r.get("agent_id")).await?;
    if agent["status"] != "active" {
        return Err(Error::new("agent_inactive", 403));
    }
    let scopes: Vec<String> =
        serde_json::from_str(r.get("scopes")).map_err(|_| Error::new("invalid_credential", 401))?;
    Ok(Agent {
        id: agent["id"].as_str().unwrap().into(),
        credential_id: r.get("id"),
        scopes: scopes
            .into_iter()
            .filter(|s| {
                agent["scopes"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!(s)))
            })
            .collect(),
    })
}
pub async fn denial(app: &App, token: &str, action: &'static str) -> Result<()> {
    let mut db = app.db.write().await?;
    let r = repository::credential(&mut db, &app.db.keys.hash(token), false).await?;
    let cred = r
        .as_ref()
        .map(|r| r.get::<String, _>("id"))
        .unwrap_or_default();
    let agent = r
        .as_ref()
        .map(|r| r.get::<String, _>("agent_id"))
        .unwrap_or_default();
    audit::record(
        &mut db,
        Event {
            actor_kind: if r.is_some() { "agent" } else { "unknown" },
            actor_id: &cred,
            agent_id: &agent,
            action,
            decision: "deny",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
pub async fn revoke(app: &App, actor: &Human, agent_id: &str, credential_id: &str) -> Result<()> {
    let mut db = app.db.write().await?;
    get(&mut db, agent_id).await?;
    let changed = repository::revoke(&mut db, agent_id, credential_id).await?;
    if changed != 1 {
        return Err(Error::new("active_credential_not_found", 404));
    }
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id,
            action: "credential.revoke",
            decision: "allow",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
pub async fn set_scopes(
    app: &App,
    actor: &Human,
    agent: &str,
    mut scopes: Vec<String>,
) -> Result<()> {
    if !scopes_valid(&scopes) {
        return Err(Error::new("invalid_request", 422));
    }
    scopes.sort();
    scopes.dedup();
    let mut db = app.db.write().await?;
    get(&mut db, agent).await?;
    repository::set_scopes(&mut db, agent, &scopes).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id: agent,
            action: "agent.scopes",
            decision: "updated",
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
