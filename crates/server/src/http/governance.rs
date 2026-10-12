use super::{
    auth,
    response::{ApiResult, json as output},
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use control_core::{App, approvals, audit, lifecycle, policy};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    decision: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enabled {
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    stopped: bool,
}
pub async fn policies(State(app): State<Arc<App>>, h: HeaderMap) -> ApiResult {
    auth::human(&app, &h, false, false).await?;
    Ok(output(policy::list(&app).await?))
}
pub async fn set_policy(
    State(app): State<Arc<App>>,
    h: HeaderMap,
    input: Result<Json<policy::Set>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &h, true, true).await?;
    Ok(output(policy::set(&app, &actor, input?.0).await?))
}
pub async fn approvals(State(app): State<Arc<App>>, h: HeaderMap) -> ApiResult {
    auth::human(&app, &h, false, false).await?;
    Ok(output(approvals::list(&app).await?))
}
pub async fn decide(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    h: HeaderMap,
    input: Result<Json<Decision>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &h, true, false).await?;
    Ok(output(
        approvals::decide(&app, &actor, &id, &input?.0.decision).await?,
    ))
}
pub async fn audit(State(app): State<Arc<App>>, h: HeaderMap) -> ApiResult {
    auth::human(&app, &h, false, false).await?;
    Ok(output(audit::list(&app.db.pool).await?))
}
pub async fn integrations(State(app): State<Arc<App>>, h: HeaderMap) -> ApiResult {
    auth::human(&app, &h, false, false).await?;
    let rows: Vec<(String, bool)> =
        sqlx::query_as("SELECT id,enabled FROM connector_state ORDER BY id")
            .fetch_all(&app.db.pool)
            .await?;
    Ok(output(json!(
        rows.into_iter()
            .map(|(id, enabled)| json!({"id":id,"enabled":enabled,"status":"registered"}))
            .collect::<Vec<_>>()
    )))
}
pub async fn connector(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
    h: HeaderMap,
    input: Result<Json<Enabled>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &h, true, true).await?;
    let enabled = input?.0.enabled;
    lifecycle::connector(&app, &actor, &name, enabled).await?;
    Ok(output(json!({"enabled":enabled})))
}
pub async fn stop(
    State(app): State<Arc<App>>,
    h: HeaderMap,
    input: Result<Json<Stop>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &h, true, true).await?;
    let stopped = input?.0.stopped;
    lifecycle::stop(&app, &actor, stopped).await?;
    Ok(output(json!({"stopped":stopped})))
}
pub async fn settings(State(app): State<Arc<App>>, h: HeaderMap) -> ApiResult {
    auth::human(&app, &h, false, false).await?;
    let stopped: bool = sqlx::query_scalar("SELECT stopped FROM emergency_state WHERE id=1")
        .fetch_one(&app.db.pool)
        .await?;
    Ok(output(
        json!({"version":"0.3.0","stage":if app.connectors.is_empty(){"foundation"}else{"github-pilot"},"default_policy":"deny","execution_enabled":app.config.execution_enabled,"emergency_stop":stopped,"external_adapters":app.connectors.len(),"registered_tools":app.connectors.iter().map(|c|c.tools().len()).sum::<usize>(),"mcp_transport":if app.config.mcp_enabled{"streamable-http"}else{"disabled"},"cancellation":"planned","health_monitoring":"planned","coverage":"Connections routed through this gateway only"}),
    ))
}
