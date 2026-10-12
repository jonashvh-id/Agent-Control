#![forbid(unsafe_code)]
pub mod agents;
pub mod approvals;
pub mod audit;
pub mod connections;
pub mod connector;
pub mod error;
pub mod execution;
pub mod gateway;
pub mod identity;
pub mod lifecycle;
pub mod models;
pub mod operations;
pub mod persistence;
pub mod policy;
pub mod resources;
pub mod security;
use std::sync::Arc;
pub struct App {
    pub db: persistence::Database,
    pub config: models::Config,
    pub connectors: Vec<Arc<dyn connector::Connector>>,
    pub(crate) dummy_hash: String,
    pub password_slots: Arc<tokio::sync::Semaphore>,
    pub provider_slots: Arc<tokio::sync::Semaphore>,
}
impl App {
    pub async fn new(
        config: models::Config,
        connectors: Vec<Arc<dyn connector::Connector>>,
    ) -> error::Result<Arc<Self>> {
        config.validate()?;
        let db = persistence::Database::open(&config.data_dir).await?;
        for c in &connectors {
            sqlx::query("INSERT OR IGNORE INTO connector_state(id,enabled) VALUES(?,0)")
                .bind(c.id())
                .execute(&db.pool)
                .await?;
        }
        let dummy_hash = security::passwords::hash(&format!(
            "UnknownAccount9!{}",
            security::keys::random_token()
        ))?;
        Ok(Arc::new(Self {
            db,
            config,
            connectors,
            dummy_hash,
            password_slots: Arc::new(tokio::sync::Semaphore::new(4)),
            provider_slots: Arc::new(tokio::sync::Semaphore::new(16)),
        }))
    }
    pub fn tool(
        &self,
        connector: &str,
        name: &str,
    ) -> error::Result<(Arc<dyn connector::Connector>, connector::Tool)> {
        self.connectors
            .iter()
            .find(|c| c.id() == connector)
            .and_then(|c| {
                c.tools()
                    .into_iter()
                    .find(|t| t.name == name)
                    .map(|t| (c.clone(), t))
            })
            .ok_or(error::Error::new("tool_not_registered", 404))
    }
    pub async fn register(
        &mut self,
        connector: Arc<dyn connector::Connector>,
    ) -> error::Result<()> {
        if self.connectors.iter().any(|c| c.id() == connector.id()) {
            return Err(error::Error::new("connector_already_registered", 409));
        }
        sqlx::query("INSERT OR IGNORE INTO connector_state(id,enabled) VALUES(?,0)")
            .bind(connector.id())
            .execute(&self.db.pool)
            .await?;
        self.connectors.push(connector);
        Ok(())
    }
}
