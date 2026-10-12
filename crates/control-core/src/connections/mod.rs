//! Provider administration port. Only separately authorized human HTTP routes expose it.
use crate::{
    error::{Error, Result},
    resources::Metadata,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub mod store;
#[derive(Clone)]
pub struct Context {
    pub human_id: String,
    pub session_hash: String,
    pub cookie_hash: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    pub state: String,
    pub code: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "iss")]
    pub issuer: Option<String>,
    pub scope: Option<String>,
    #[serde(rename = "authuser")]
    pub auth_user: Option<String>,
    pub prompt: Option<String>,
}
#[derive(Serialize)]
pub struct Start {
    pub authorization_url: String,
}
#[async_trait::async_trait]
pub trait Management: Send + Sync {
    async fn status(&self) -> Result<Value>;
    async fn begin(&self, context: Context) -> Result<Start>;
    async fn finish(&self, cookie_hash: String, response: Callback) -> Result<String>;
    async fn disconnect(&self) -> Result<Value>;
    async fn inspect_resource(&self, _provider_id: &str) -> Result<Metadata> {
        Err(Error::new("resource_configuration_unavailable", 503))
    }
    async fn verify_extent(&self) -> Result<Value> {
        Err(Error::new("native_verification_unavailable", 503))
    }
    async fn invalidate_extent(&self) -> Result<()> {
        Ok(())
    }
    async fn candidates(&self) -> Result<Value> {
        Err(Error::new("resource_configuration_unavailable", 503))
    }
}
