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
    lifecycle::check(db, connector).await?;
    let mode = policy::current(db, &actor, connector, tool).await?;
    if mode == Policy::Deny {
        return Err(Error::new("policy_denied", 403));
    }
    if !app.config.execution_enabled {
        return Err(Error::new("execution_disabled", 503));
    }
    Ok((actor, mode))
}
