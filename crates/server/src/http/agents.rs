use super::{
    auth,
    response::{ApiResult, json as output},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use control_core::{App, agents, lifecycle};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    status: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scopes {
    #[serde(default)]
    scopes: Vec<String>,
}
pub async fn list(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    Ok(output(agents::list(&app).await?))
}
pub async fn create(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    input: Result<Json<agents::Create>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    Ok((
        StatusCode::CREATED,
        Json(agents::create(&app, &actor, input?.0).await?),
    )
        .into_response())
}
pub async fn details(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    let mut db = app.db.pool.acquire().await?;
    let mut agent = agents::get(&mut db, &id).await?;
    agent["credentials"] = agents::credentials(&mut db, &id).await?;
    Ok(output(agent))
}
pub async fn status(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Status>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let status = input?.0.status;
    lifecycle::agent(&app, &actor, &id, &status).await?;
    Ok(output(json!({"status":status})))
}
pub async fn issue(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Scopes>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    Ok((
        StatusCode::CREATED,
        Json(agents::issue(&app, &actor, &id, input?.0.scopes).await?),
    )
        .into_response())
}
pub async fn revoke(
    State(app): State<Arc<App>>,
    Path((id, credential)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    agents::revoke(&app, &actor, &id, &credential).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
