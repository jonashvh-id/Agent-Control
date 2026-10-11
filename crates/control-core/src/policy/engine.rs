use crate::{connector::Tool, models::Policy};
pub fn effective(tool: &Tool, scopes: &[String], explicit: Policy) -> Policy {
    if !tool.required_scopes.iter().all(|s| scopes.contains(s)) {
        Policy::Deny
    } else if explicit == Policy::Allow && tool.requires_confirmation {
        Policy::Ask
    } else {
        explicit
    }
}
