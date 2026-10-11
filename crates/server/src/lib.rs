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
    let url = config::validate_origin(&config)?;
    let mut connectors: Vec<Arc<dyn Connector>> = vec![];
    if let Some(config) = github {
        connectors.push(Arc::new(connector_github::GitHub::new(
            config,
            matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        )?));
    }
    App::new(config, connectors).await
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
