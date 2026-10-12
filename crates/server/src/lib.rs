#![forbid(unsafe_code)]
pub mod assets;
pub mod bootstrap;
pub mod cli;
pub mod config;
pub use bootstrap::run;
pub mod http;
pub mod mcp;
use control_core::{App, connector::Connector, error::Result, models::Config};
use std::sync::Arc;
pub async fn compose(config: Config, github: Option<connector_github::Config>) -> Result<Arc<App>> {
    compose_with_drive(config, github, None).await
}
pub async fn compose_with_drive(
    config: Config,
    github: Option<connector_github::Config>,
    drive: Option<connector_google_drive::Config>,
) -> Result<Arc<App>> {
    compose_with_oauth(config, github, drive, None).await
}
pub async fn compose_with_oauth(
    config: Config,
    github: Option<connector_github::Config>,
    drive: Option<connector_google_drive::Config>,
    mut oauth: Option<connector_google_drive::oauth::Config>,
) -> Result<Arc<App>> {
    if drive.is_some() && oauth.is_some() {
        return Err(control_core::error::Error::new(
            "google_credential_modes_conflict",
            400,
        ));
    }
    let url = config::validate_origin(&config)?;
    let mut connectors: Vec<Arc<dyn Connector>> = vec![];
    if let Some(config) = github {
        connectors.push(Arc::new(connector_github::GitHub::new(
            config,
            matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        )?));
    }
    if let Some(config) = drive {
        connectors.push(Arc::new(connector_google_drive::Drive::new(config)?));
    }
    if let Some(oauth) = oauth.as_mut() {
        oauth.origin = config.origin.clone();
    }
    let mut app = App::new(config, connectors).await?;
    if let Some(config) = oauth {
        let store = control_core::connections::store::Store {
            pool: app.db.pool.clone(),
            keys: app.db.keys.clone(),
        };
        let manager = connector_google_drive::oauth::Manager::new(config, store)?;
        Arc::get_mut(&mut app)
            .expect("composition has exclusive ownership")
            .register(Arc::new(
                connector_google_drive::managed::ManagedDrive::new(manager),
            ))
            .await?;
    }
    Ok(app)
}
pub fn router(app: Arc<App>) -> axum::Router {
    let mut router = http::router();
    if app.config.mcp_enabled {
        router = router.merge(mcp::router(app.clone()));
    }
    router
        .fallback(assets::serve)
        .layer(axum::middleware::from_fn_with_state(
            app.clone(),
            http::boundary::guard,
        ))
        .with_state(app)
}
