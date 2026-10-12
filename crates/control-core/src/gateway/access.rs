//! Human-only diagnostics; execution still requires a current bearer credential.
use crate::{
    App, agents,
    error::Result,
    gateway::authorization,
    models::{Agent, Policy},
    policy,
};
use serde_json::{Value, json};
pub async fn describe(app: &App, id: &str) -> Result<Value> {
    let mut db = app.db.write().await?;
    let agent = agents::get(&mut db, id).await?;
    let credentials = agents::credentials(&mut db, id).await?;
    let scopes: Vec<String> = serde_json::from_value(agent["scopes"].clone()).unwrap_or_default();
    let mut tools = vec![];
    for connector in &app.connectors {
        for tool in connector.tools() {
            let explicit =
                policy::repository::mode(&mut db, id, connector.id(), &tool.name).await?;
            let mut reason = if agent["status"] != "active" {
                "agent_inactive"
            } else {
                "no_active_credential"
            };
            let mut effective = Policy::Deny;
            let mut grants = vec![];
            let mut resources = crate::resources::available(&mut db, id, connector.id()).await?;
            crate::resources::labels(&mut db, &app.db.keys, &mut resources).await?;
            for credential in credentials
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["status"] == "active")
            {
                if agent["status"] != "active" {
                    break;
                }
                let assigned: Vec<String> =
                    serde_json::from_value(credential["scopes"].clone()).unwrap_or_default();
                let actor = Agent {
                    id: id.into(),
                    credential_id: credential["id"].as_str().unwrap().into(),
                    scopes: assigned
                        .into_iter()
                        .filter(|s| scopes.contains(s))
                        .collect(),
                };
                let result =
                    authorization::for_actor(app, &mut db, &actor, connector.id(), &tool).await;
                match result {
                    Ok(mode) => {
                        effective = mode;
                        reason = "authorized";
                        grants.push(json!({"credential_id":actor.credential_id,"mode":mode}));
                    }
                    Err(error) if grants.is_empty() => {
                        reason = if error.code == "policy_denied"
                            && !tool
                                .required_scopes
                                .iter()
                                .all(|s| actor.scopes.contains(s))
                        {
                            "scope_missing"
                        } else if error.code == "policy_denied" && explicit.is_none() {
                            "default_deny"
                        } else {
                            error.code
                        };
                    }
                    Err(_) => (),
                }
            }
            tools.push(json!({"connector":connector.id(),"tool":tool.name,"description":tool.description,
                "required_scopes":tool.required_scopes,"explicit_policy":explicit.unwrap_or("deny".into()),
                "effective_policy":effective,"reason":reason,"requires_confirmation":tool.requires_confirmation,
                "authorized_credentials":grants,"resources":resources,"resource_permissions_required":connector.requires_resources()}));
        }
    }
    db.commit().await?;
    Ok(json!({"agent_id":id,"tools":tools}))
}
