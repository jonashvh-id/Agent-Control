use super::{
    auth,
    connections::manager,
    response::{ApiResult, error, json as output},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use control_core::{
    App,
    audit::{self, Event},
    models::Policy,
    persistence::now,
    resources::{self, Store},
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Add {
    connector: String,
    provider_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enabled {
    enabled: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permission {
    agent_id: String,
    resource_id: String,
    mode: Policy,
}
fn store(app: &App) -> Store {
    Store {
        pool: app.db.pool.clone(),
        keys: app.db.keys.clone(),
    }
}
pub async fn list(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    let mut resources = vec![];
    for c in &app.connectors {
        resources.extend(store(&app).list(c.id()).await?);
    }
    Ok(output(json!(resources)))
}
pub async fn add(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    input: Result<Json<Add>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let input = input?.0;
    let control = manager(&app, &input.connector)?;
    let metadata = control.inspect_resource(&input.provider_id).await?;
    let id = store(&app)
        .add(&input.connector, &actor.id, metadata)
        .await?;
    control.invalidate_extent().await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"id":id,"native_verification_required":true})),
    )
        .into_response())
}
pub async fn enable(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Enabled>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let enabled = input?.0.enabled;
    let mut db = app.db.write().await?;
    let connector: Option<String> =
        sqlx::query_scalar("SELECT connector FROM resources WHERE id=?")
            .bind(&id)
            .fetch_optional(&mut *db)
            .await?;
    let connector = connector.ok_or_else(|| error("resource_not_found", 404))?;
    if enabled {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM resources WHERE connector=? AND enabled=1 AND id!=?",
        )
        .bind(&connector)
        .bind(&id)
        .fetch_one(&mut *db)
        .await?;
        if count >= 20 {
            return Err(error("resource_limit_reached", 409));
        }
    }
    sqlx::query("UPDATE resources SET enabled=?,updated_at=? WHERE id=?")
        .bind(enabled)
        .bind(now())
        .bind(&id)
        .execute(&mut *db)
        .await?;
    sqlx::query("UPDATE connection_secrets SET verification='{}',resource_revision=resource_revision+1 WHERE connector=?")
        .bind(&connector)
        .execute(&mut *db)
        .await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: "human",
            actor_id: &actor.id,
            connector: &connector,
            resource_ref: &id,
            action: "resource.status",
            decision: if enabled { "enabled" } else { "disabled" },
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
pub async fn policies(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    let rows = sqlx::query("SELECT * FROM resource_policies ORDER BY updated_at DESC")
        .fetch_all(&app.db.pool)
        .await?;
    Ok(output(json!(
        rows.iter()
            .map(control_core::persistence::row_json)
            .collect::<Vec<_>>()
    )))
}
pub async fn set_policy(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    input: Result<Json<Permission>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let p = input?.0;
    resources::policy::set(&app, &actor, &p.agent_id, &p.resource_id, p.mode).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
pub async fn operations(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    Ok(output(control_core::operations::dashboard(&app).await?))
}
