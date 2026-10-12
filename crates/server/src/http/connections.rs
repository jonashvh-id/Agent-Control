use super::{
    auth,
    response::{ApiResult, error, json as output},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Redirect},
};
use control_core::{
    App,
    audit::{self, Event},
    connections::{Callback, Context, Management},
    security::keys::random_token,
};
use std::sync::Arc;
pub(crate) fn manager(
    app: &App,
    id: &str,
) -> Result<Arc<dyn Management>, super::response::ApiError> {
    app.connectors
        .iter()
        .find(|c| c.id() == id)
        .and_then(|c| c.management())
        .ok_or_else(|| error("oauth_not_configured", 503))
}
fn flow_cookie(h: &HeaderMap) -> Option<&str> {
    let values = h
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|v| v.trim().strip_prefix("ac_google_oauth="))
        .collect::<Vec<_>>();
    (values.len() == 1).then(|| values[0])
}
fn cookie(value: &str, secure: bool, max_age: u16) -> HeaderValue {
    HeaderValue::from_str(&format!("ac_google_oauth={value}; Path=/api/v1/connections/google-drive/callback; HttpOnly; SameSite=Lax; Max-Age={max_age}{}",if secure{"; Secure"}else{""})).unwrap()
}
async fn event(
    app: &App,
    actor: &str,
    action: &str,
    decision: &str,
) -> Result<(), control_core::error::Error> {
    let mut db = app.db.write().await?;
    audit::record(
        &mut db,
        Event {
            actor_kind: if actor.is_empty() { "unknown" } else { "human" },
            actor_id: actor,
            connector: "google-drive",
            action,
            decision,
            ..Default::default()
        },
    )
    .await?;
    db.commit().await?;
    Ok(())
}
pub async fn status(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    if let Ok(manager) = manager(&app, &id) {
        return Ok(output(manager.status().await?));
    }
    if id != "google-drive" {
        return Err(error("connector_not_found", 404));
    }
    Ok(output(
        serde_json::json!({"state":"oauth_not_configured","automatic_refresh":false,"redirect_uri":format!("{}/api/v1/connections/google-drive/callback",app.config.origin),"configuration":"Register a Web OAuth client in your own Google Cloud project. Set AGENT_CONTROL_GOOGLE_OAUTH_CLIENT_ID and the private AGENT_CONTROL_GOOGLE_OAUTH_SECRET_FILE; restart this server. Use a dedicated reader identity. No user access token is configured manually."}),
    ))
}
pub async fn begin(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let manager = manager(&app, &id)?;
    let nonce = random_token();
    let start = manager
        .begin(Context {
            human_id: actor.id.clone(),
            session_hash: app.db.keys.hash(&auth::cookie(&headers)),
            cookie_hash: app.db.keys.hash(&nonce),
        })
        .await?;
    event(&app, &actor.id, "connection.oauth_started", "allow").await?;
    let mut response = Json(start).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie(&nonce, app.config.secure_cookies, 600),
    );
    Ok(response)
}
pub async fn callback(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    input: Result<Query<Callback>, axum::extract::rejection::QueryRejection>,
) -> ApiResult {
    let response = input.map_err(|_| error("invalid_oauth_callback", 400))?.0;
    let flow = flow_cookie(&headers)
        .filter(|s| s.len() <= 128)
        .ok_or_else(|| error("oauth_state_rejected", 403))?;
    let manager = manager(&app, "google-drive")?;
    let result = manager.finish(app.db.keys.hash(flow), response).await;
    if result.is_err() {
        event(&app, "", "connection.oauth_refused", "deny").await?;
    }
    let mut response = Redirect::to(if result.is_ok() {
        "/?google=connected#integrations"
    } else {
        "/?google=connection_failed#integrations"
    })
    .into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie("", app.config.secure_cookies, 0));
    Ok(response)
}
pub async fn disconnect(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let result = manager(&app, &id)?.disconnect().await?;
    event(&app, &actor.id, "connection.disconnected", "allow").await?;
    Ok(output(result))
}
pub async fn verify(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult {
    let actor = auth::human(&app, &headers, true, true).await?;
    let result = manager(&app, &id)?.verify_extent().await?;
    event(
        &app,
        &actor.id,
        "connection.native_extent_checked",
        if result["verified"] == true {
            "allow"
        } else {
            "deny"
        },
    )
    .await?;
    Ok(output(result))
}
pub async fn candidates(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult {
    auth::human(&app, &headers, false, true).await?;
    Ok(output(manager(&app, &id)?.candidates().await?))
}
