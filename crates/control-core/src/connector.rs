use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub required_scopes: Vec<String>,
    pub risk_level: String,
    pub requires_confirmation: bool,
}
#[async_trait::async_trait]
pub trait Connector: Send + Sync {
    fn management(&self) -> Option<std::sync::Arc<dyn crate::connections::Management>> {
        None
    }
    fn requires_resources(&self) -> bool {
        false
    }
    fn observations(&self, _result: &Value) -> Vec<crate::resources::Metadata> {
        vec![]
    }
    fn id(&self) -> &'static str;
    fn tools(&self) -> Vec<Tool>;
    fn validate(&self, tool: &str, parameters: &Value) -> Result<()>;
    fn preview(&self, tool: &str, parameters: &Value) -> Option<Value>;
    async fn execute(&self, tool: &str, parameters: &Value, execution_id: &str) -> Result<Value>;
    fn receipt(&self, result: &Value) -> Option<Value>;
}
