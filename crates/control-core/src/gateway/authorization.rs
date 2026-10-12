use crate::{
    App, agents,
    connector::Tool,
    error::{Error, Result},
    lifecycle,
    models::{Agent, Policy},
    policy,
};
pub async fn check(
    app: &App,
    db: &mut sqlx::SqliteConnection,
    token: &str,
    connector: &str,
    tool: &Tool,
) -> Result<(Agent, Policy)> {
    let actor = agents::resolve(app, db, token).await?;
    let mode = for_actor(app, db, &actor, connector, tool).await?;
    Ok((actor, mode))
}
// The caller must have resolved a current actor. Also used by human-only access diagnostics.
pub async fn for_actor(
    app: &App,
    db: &mut sqlx::SqliteConnection,
    actor: &Agent,
    connector: &str,
    tool: &Tool,
) -> Result<Policy> {
    lifecycle::check(db, connector).await?;
    let mode = policy::current(db, actor, connector, tool).await?;
    if mode == Policy::Deny {
        return Err(Error::new("policy_denied", 403));
    }
    if !app.config.execution_enabled {
        return Err(Error::new("execution_disabled", 503));
    }
    Ok(mode)
}
