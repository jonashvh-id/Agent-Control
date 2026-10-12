pub mod engine;
pub mod repository;
use crate::{
    App,
    audit::{self, Event},
    connector::Tool,
    error::{Error, Result},
    models::{Agent, Human, Policy},
};
pub use engine::effective;
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Set {
    pub agent_id: String,
    pub connector: String,
    pub tool: String,
    pub mode: Policy,
}
pub async fn current(
    db: &mut sqlx::SqliteConnection,
    actor: &Agent,
    connector: &str,
    tool: &Tool,
) -> Result<Policy> {
    let mode = repository::mode(db, &actor.id, connector, &tool.name).await?;
    let policy = match mode.as_deref() {
        Some("allow") => Policy::Allow,
        Some("ask") => Policy::Ask,
        _ => Policy::Deny,
    };
    Ok(effective(tool, &actor.scopes, policy))
}
pub async fn set(app: &App, actor: &Human, input: Set) -> Result<Value> {
    if !regex::Regex::new(r"^[a-f0-9]{32}$")
        .unwrap()
        .is_match(&input.agent_id)
        || !regex::Regex::new(r"^[a-z][a-z0-9_-]{0,63}$")
            .unwrap()
            .is_match(&input.connector)
        || !regex::Regex::new(r"^[a-z][a-z0-9_.:-]{0,127}$")
            .unwrap()
            .is_match(&input.tool)
    {
        return Err(Error::new("invalid_request", 422));
    }
    let mut db = app.db.write().await?;
    crate::agents::get(&mut db, &input.agent_id).await?;
    repository::set(&mut db, &actor.id, &input).await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            agent_id: &input.agent_id,
            connector: &input.connector,
            tool: &input.tool,
            action: "policy.set",
            decision: input.mode.as_str(),
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(
        json!({"agent_id":input.agent_id,"connector":input.connector,"tool":input.tool,"mode":input.mode,"tool_registered":app.tool(&input.connector,&input.tool).is_ok()}),
    )
}
pub async fn list(app: &App) -> Result<Value> {
    repository::list(&app.db.pool).await
}
