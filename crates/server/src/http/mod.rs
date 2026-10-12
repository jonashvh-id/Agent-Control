pub mod agents;
pub mod auth;
pub mod boundary;
pub mod catalog;
pub mod connections;
pub mod gateway;
pub mod governance;
pub mod resources;
pub mod response;
use axum::{
    Json, Router,
    extract::State,
    routing::{delete, get, patch, post, put},
};
use control_core::App;
use serde_json::json;
use std::sync::Arc;
pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/agents", get(agents::list).post(agents::create))
        .route("/api/v1/agents/{id}", get(agents::details))
        .route("/api/v1/agents/{id}/access", get(agents::access))
        .route("/api/v1/agents/{id}/scopes", patch(agents::scopes))
        .route("/api/v1/catalog", get(catalog::list))
        .route(
            "/api/v1/connections/{id}",
            get(connections::status).delete(connections::disconnect),
        )
        .route("/api/v1/connections/{id}/start", post(connections::begin))
        .route("/api/v1/connections/{id}/verify", post(connections::verify))
        .route(
            "/api/v1/connections/{id}/folders",
            get(connections::candidates),
        )
        .route(
            "/api/v1/connections/google-drive/callback",
            get(connections::callback),
        )
        .route(
            "/api/v1/resources",
            get(resources::list).post(resources::add),
        )
        .route("/api/v1/resources/{id}", patch(resources::enable))
        .route(
            "/api/v1/resource-policies",
            get(resources::policies).put(resources::set_policy),
        )
        .route("/api/v1/operations", get(resources::operations))
        .route("/api/v1/agents/{id}/status", patch(agents::status))
        .route("/api/v1/agents/{id}/credentials", post(agents::issue))
        .route(
            "/api/v1/agents/{id}/credentials/{credential}",
            delete(agents::revoke),
        )
        .route(
            "/api/v1/policies",
            get(governance::policies).put(governance::set_policy),
        )
        .route("/api/v1/approvals", get(governance::approvals))
        .route("/api/v1/approvals/{id}/decision", post(governance::decide))
        .route("/api/v1/audit", get(governance::audit))
        .route("/api/v1/integrations", get(governance::integrations))
        .route(
            "/api/v1/integrations/{connector}",
            patch(governance::connector),
        )
        .route("/api/v1/settings", get(governance::settings))
        .route("/api/v1/security/emergency-stop", put(governance::stop))
        .route("/api/v1/gateway/tools", get(gateway::tools))
        .route("/api/v1/gateway/execute", post(gateway::execute))
}
async fn health(State(app): State<Arc<App>>) -> response::ApiResult {
    sqlx::query("SELECT 1")
        .execute(&app.db.pool)
        .await
        .map_err(control_core::error::Error::from)?;
    Ok(Json(json!({"status":"ok","service":"agent-control","stage":"foundation"})).into_response())
}
use axum::response::IntoResponse;
