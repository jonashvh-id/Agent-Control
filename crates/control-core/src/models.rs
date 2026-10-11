use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
#[derive(Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub origin: String,
    pub secure_cookies: bool,
    pub execution_enabled: bool,
    pub mcp_enabled: bool,
    pub session_seconds: u64,
    pub approval_seconds: u64,
}
impl Config {
    pub fn local(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            origin: "http://127.0.0.1:8010".into(),
            secure_cookies: false,
            execution_enabled: false,
            mcp_enabled: true,
            session_seconds: 28800,
            approval_seconds: 900,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.session_seconds == 0
            || self.session_seconds > 86400
            || self.approval_seconds == 0
            || self.approval_seconds > 3600
        {
            return Err(Error::new("invalid_configuration", 400));
        }
        Ok(())
    }
}
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    Deny,
    Ask,
    Allow,
}
impl Policy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::Ask => "ask",
            Self::Allow => "allow",
        }
    }
}
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Prepared,
    Dispatched,
    Executed,
    Failed,
    Uncertain,
}
impl ExecutionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Dispatched => "dispatched",
            Self::Executed => "executed",
            Self::Failed => "failed",
            Self::Uncertain => "uncertain",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Human {
    pub id: String,
    pub username: String,
    pub role: String,
    pub csrf: String,
}
#[derive(Clone, Debug)]
pub struct Agent {
    pub id: String,
    pub credential_id: String,
    pub scopes: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub connector: String,
    pub tool: String,
    #[serde(default)]
    pub parameters: serde_json::Map<String, Value>,
    pub idempotency_key: String,
    pub approval_id: Option<String>,
}
impl Proposal {
    pub fn validate(&self) -> Result<()> {
        let matches = |value: &str, pattern: &str| {
            regex::Regex::new(pattern)
                .expect("static regex")
                .is_match(value)
        };
        if !matches(&self.connector, r"^[a-z][a-z0-9_-]{0,63}$")
            || !matches(&self.tool, r"^[a-z][a-z0-9_.:-]{0,127}$")
            || !matches(&self.idempotency_key, r"^[A-Za-z0-9_-]{8,128}$")
            || self
                .approval_id
                .as_ref()
                .is_some_and(|v| !matches(v, r"^[a-f0-9]{32}$"))
        {
            return Err(Error::new("invalid_request", 422));
        }
        let parameters = Value::Object(self.parameters.clone());
        let mut stack = vec![(&parameters, 0)];
        let mut count = 0;
        while let Some((value, depth)) = stack.pop() {
            count += 1;
            if depth > 32 || count > 1000 {
                return Err(Error::new("invalid_request", 422));
            }
            match value {
                Value::Array(a) => stack.extend(a.iter().map(|v| (v, depth + 1))),
                Value::Object(o) => stack.extend(o.values().map(|v| (v, depth + 1))),
                _ => (),
            }
        }
        if parameters.to_string().len() > 16384 {
            return Err(Error::new("invalid_request", 422));
        }
        Ok(())
    }
    pub fn binding(&self) -> Value {
        serde_json::json!({"connector":self.connector,"tool":self.tool,"parameters":self.parameters,"idempotency_key":self.idempotency_key})
    }
}
