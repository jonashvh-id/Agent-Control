pub mod redaction;
pub mod repository;
pub use redaction::summarize;
pub use repository::{list, record};
use serde_json::Value;
#[derive(Default)]
pub struct Event<'a> {
    pub actor_kind: &'a str,
    pub actor_id: &'a str,
    pub agent_id: &'a str,
    pub connector: &'a str,
    pub tool: &'a str,
    pub action: &'a str,
    pub decision: &'a str,
    pub approval_id: &'a str,
    pub execution_id: &'a str,
    pub correlation_id: &'a str,
    pub input_hash: &'a str,
    pub parameters: Option<&'a Value>,
    pub receipt: Option<&'a Value>,
}
