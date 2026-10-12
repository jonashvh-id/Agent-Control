pub mod config;
mod flow;
pub(crate) mod http;
mod tokens;
pub use config::Config;
use control_core::{connections::store::Store, error::Result, resources};
use std::sync::Arc;
pub struct Manager {
    pub(crate) config: Config,
    pub(crate) store: Store,
    pub(crate) resources: resources::Store,
    pub(crate) http: http::Http,
    pub(crate) origin: String,
    pub(crate) token_endpoint: String,
    pub(crate) revoke_endpoint: String,
    pub(crate) lock: tokio::sync::Mutex<()>,
    pub(crate) extent_lock: tokio::sync::Mutex<()>,
}
#[cfg(test)]
mod tests;
impl Manager {
    pub fn new(config: Config, store: Store) -> Result<Arc<Self>> {
        config.validate()?;
        let resources = resources::Store {
            pool: store.pool.clone(),
            keys: store.keys.clone(),
        };
        Ok(Arc::new(Self {
            config,
            store,
            resources,
            http: http::Http::new()?,
            origin: "https://www.googleapis.com".into(),
            token_endpoint: "https://oauth2.googleapis.com/token".into(),
            revoke_endpoint: "https://oauth2.googleapis.com/revoke".into(),
            lock: tokio::sync::Mutex::new(()),
            extent_lock: tokio::sync::Mutex::new(()),
        }))
    }
}
