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
                if c.requires_resources() {
                    let actor = crate::agents::resolve(app, &mut db, token).await?;
                    let mut resources =
                        crate::resources::available(&mut db, &actor.id, c.id()).await?;
                    crate::resources::labels(&mut db, &app.db.keys, &mut resources).await?;
                    value["input_schema"]["properties"]["resource_id"]["enum"] = serde_json::json!(
                        resources
                            .iter()
                            .map(|r| r["id"].clone())
                            .collect::<Vec<_>>()
                    );
                    value["resources"] = serde_json::json!(resources);
                    value["input_schema"]["properties"]["resource_id"]["oneOf"]=serde_json::json!(resources.iter().map(|r|serde_json::json!({"const":r["id"],"title":r["name"].as_str().unwrap_or("").chars().take(120).collect::<String>()})).collect::<Vec<_>>());
                }
                tools.push(value);
            }
        }
    }
    Ok(tools)
}
