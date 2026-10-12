use super::{
    auth,
    response::{ApiResult, json as output},
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use control_core::{App, gateway, models::Proposal};
use serde_json::json;
use std::sync::Arc;
pub async fn tools(State(app): State<Arc<App>>, h: HeaderMap) -> ApiResult {
    let token = auth::agent(&app, &h).await?;
    Ok(output(
        json!({"tools":gateway::discover(&app,&token).await?,"mcp_transport":if app.config.mcp_enabled{"streamable-http"}else{"disabled"}}),
    ))
}
pub async fn execute(
    State(app): State<Arc<App>>,
    h: HeaderMap,
    input: Result<Json<Proposal>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let token = auth::agent(&app, &h).await?;
    let result = gateway::execute(&app, &token, input?.0).await?;
    Ok((
        StatusCode::from_u16(result.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(result.body),
    )
        .into_response())
}
