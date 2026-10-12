use super::{
    auth,
    response::{ApiResult, json as output},
};
use axum::{extract::State, http::HeaderMap};
use control_core::App;
use serde_json::json;
use std::sync::Arc;
pub async fn list(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    auth::human(&app, &headers, false, false).await?;
    let mut connectors = vec![];
    for c in &app.connectors {
        let enabled: bool = sqlx::query_scalar("SELECT enabled FROM connector_state WHERE id=?")
            .bind(c.id())
            .fetch_one(&app.db.pool)
            .await?;
        connectors.push(json!({"id":c.id(),"enabled":enabled,"tools":c.tools(),"resource_permissions_required":c.requires_resources()}));
    }
    Ok(output(
        json!({"connectors":connectors,"mcp_url":format!("{}/mcp/",app.config.origin),
        "mcp_enabled":app.config.mcp_enabled,"execution_enabled":app.config.execution_enabled}),
    ))
}
