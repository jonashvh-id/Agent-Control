use crate::{
    audit::{self, Event},
    error::Result,
    models::{Agent, Proposal},
};
use serde_json::Value;
pub struct Journal<'a> {
    pub known: bool,
    pub actor: Option<&'a Agent>,
    pub proposal: &'a Proposal,
    pub correlation: &'a str,
    pub hash: &'a str,
    pub execution_id: &'a str,
    pub approval_id: &'a str,
}
impl Journal<'_> {
    pub async fn event(
        &self,
        db: &mut sqlx::SqliteConnection,
        action: &str,
        decision: &str,
        receipt: Option<&Value>,
    ) -> Result<()> {
        audit::record(
            db,
            Event {
                actor_kind: if self.actor.is_some() {
                    "agent"
                } else {
                    "unknown"
                },
                actor_id: self.actor.map_or("", |a| a.credential_id.as_str()),
                agent_id: self.actor.map_or("", |a| a.id.as_str()),
                connector: if self.known {
                    &self.proposal.connector
                } else {
                    ""
                },
                tool: if self.known { &self.proposal.tool } else { "" },
                action,
                decision,
                approval_id: self.approval_id,
                execution_id: self.execution_id,
                correlation_id: self.correlation,
                input_hash: self.hash,
                parameters: Some(&Value::Object(self.proposal.parameters.clone())),
                receipt,
            },
        )
        .await
    }
}
