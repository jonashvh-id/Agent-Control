pub mod access;
pub mod admission;
pub mod authorization;
pub mod dispatch;
pub mod journal;
use crate::{App, error::Result, models::Proposal};
use serde_json::Value;
#[derive(Debug)]
pub struct Outcome {
    pub status: u16,
    pub body: Value,
}
pub async fn execute(app: &App, token: &str, p: Proposal) -> Result<Outcome> {
    p.validate()?;
    let correlation = crate::persistence::id();
    match admission::prepare(app, token, &p, &correlation).await? {
        admission::Admission::Outcome(outcome) => Ok(outcome),
        admission::Admission::Prepared(prepared) => {
            dispatch::execute(app, token, &p, &correlation, *prepared).await
        }
    }
}
pub async fn discover(app: &App, token: &str) -> Result<Vec<Value>> {
    let mut db = app.db.pool.acquire().await?;
    crate::agents::resolve(app, &mut db, token).await?;
    let mut tools = vec![];
    for c in &app.connectors {
        for tool in c.tools() {
            if let Ok((_, mode)) = authorization::check(app, &mut db, token, c.id(), &tool).await {
                let mut value = serde_json::to_value(&tool).expect("tool serialization");
                value["connector"] = serde_json::json!(c.id());
                value["policy"] = serde_json::json!(mode);
                value["requires_confirmation"] =
                    serde_json::json!(mode == crate::models::Policy::Ask);
                tools.push(value);
            }
        }
    }
    Ok(tools)
}
