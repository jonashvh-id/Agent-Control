use crate::{
    App, agents, approvals,
    connector::{Connector, Tool},
    error::{Error, Result},
    gateway::{Outcome, authorization, journal::Journal},
    models::{Agent, Policy, Proposal},
    persistence::id,
};
use serde_json::{Value, json};

use std::sync::Arc;
pub struct Prepared {
    pub(super) connector: Arc<dyn Connector>,
    pub(super) tool: Tool,
    pub(super) actor: Agent,
    pub(super) hash: String,
    pub(super) approval_id: String,
    pub(super) execution_id: String,
}
pub enum Admission {
    Outcome(Outcome),
    Prepared(Box<Prepared>),
}
pub async fn prepare(app: &App, token: &str, p: &Proposal, correlation: &str) -> Result<Admission> {
    let hash = app.db.keys.fingerprint(&p.binding());
    let mut db = app.db.write().await?;
    let mut actor = None;
    let mut known = false;
    let mut mode = Policy::Deny;
    let mut approval = p.approval_id.clone().unwrap_or_default();
    let mut execution = String::new();
    let result: Result<Admission> = async {
        actor = Some(agents::resolve(app, &mut db, token).await?);
        let (connector, tool) = app.tool(&p.connector, &p.tool)?;
        known = true;
        let (resolved, effective) = authorization::check(app, &mut db, token, &p.connector, &tool).await?;
        actor = Some(resolved.clone());
        mode = effective;
        let parameters = Value::Object(p.parameters.clone());
        let validator = jsonschema::validator_for(&tool.input_schema)
            .map_err(|_| Error::new("invalid_tool_schema", 503))?;
        if !validator.is_valid(&parameters) {
            return Err(Error::new("invalid_tool_parameters", 422));
        }
        connector.validate(&p.tool, &parameters)?;
        let previous=crate::execution::previous(&mut db,&resolved.id,&p.idempotency_key,&hash).await?;
        if let Some(record)=previous {
            let status=record.state.as_str();
            execution=record.id;
            let receipt=record.receipt;
            Journal { known, actor: actor.as_ref(), proposal: p, correlation, hash: &hash,
                execution_id: &execution, approval_id: &approval }
                .event(&mut db, "execution.replay", mode.as_str(), None).await?;
            return Ok(Admission::Outcome(Outcome {
                status: if status == "executed" { 200 } else { 409 },
                body: json!({
                    "status": if status == "executed" { "already_executed" } else { "execution_uncertain" },
                    "execution_state": status, "execution_id": execution,
                    "result_available": false,
                    "receipt": receipt,
                    "correlation_id": correlation
                }),
            }));
        }
        if !approval.is_empty() {
            approvals::consume(&mut db, &approval, &resolved, p, &hash).await?;
        } else if mode == Policy::Ask {
            approval = approvals::create(app, &mut db, &resolved, p, &hash,
                connector.preview(&p.tool, &parameters)).await?;
            Journal { known, actor: actor.as_ref(), proposal: p, correlation, hash: &hash,
                execution_id: "", approval_id: &approval }
                .event(&mut db, "approval.requested", mode.as_str(), None).await?;
            return Ok(Admission::Outcome(Outcome {
                status: 202,
                body: json!({"status": "requires_approval", "approval_id": approval,
                    "correlation_id": correlation}),
            }));
        }
        execution = id();
        crate::execution::claim(&mut db,&execution,&resolved.id,&p.idempotency_key,&hash,if approval.is_empty(){None}else{Some(&approval)}).await?;
        Journal { known, actor: actor.as_ref(), proposal: p, correlation, hash: &hash,
            execution_id: &execution, approval_id: &approval }
            .event(&mut db, "execution.prepared", mode.as_str(), None).await?;
        Ok(Admission::Prepared(Box::new(Prepared {
            connector, tool, actor: resolved, hash: hash.clone(),
            approval_id: approval.clone(), execution_id: execution.clone(),
        })))
    }.await;
    let outcome = match result {
        Ok(v) => v,
        Err(error) => {
            Journal {
                known,
                actor: actor.as_ref(),
                proposal: p,
                correlation,
                hash: &hash,
                execution_id: &execution,
                approval_id: &approval,
            }
            .event(&mut db, error.code, "deny", None)
            .await?;
            Admission::Outcome(Outcome {
                status: error.status,
                body: json!({"error":error.code,"correlation_id":correlation}),
            })
        }
    };
    db.commit().await?;
    Ok(outcome)
}
