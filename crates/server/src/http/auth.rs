use super::response::{ApiError, ApiResult, error};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use control_core::{App, agents, identity, models::Human};
use serde::Deserialize;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Login {
    username: String,
    password: String,
}
pub fn cookie(headers: &HeaderMap) -> String {
    headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .split(';')
        .find_map(|v| v.trim().strip_prefix("ac_session="))
        .unwrap_or("")
        .into()
}
pub fn bearer(headers: &HeaderMap) -> Result<String, ApiError> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(error("agent_authentication_required", 401));
    }
    let header = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let mut parts = header.split(' ');
    let (Some(scheme), Some(token)) = (parts.next(), parts.next()) else {
        return Err(error("agent_authentication_required", 401));
    };
    if !scheme.eq_ignore_ascii_case("bearer")
        || token.is_empty()
        || token.len() > 256
        || token.chars().any(char::is_whitespace)
        || parts.next().is_some()
    {
        return Err(error("agent_authentication_required", 401));
    }
    Ok(token.into())
}
pub async fn agent(app: &App, headers: &HeaderMap) -> Result<String, ApiError> {
    let token = match bearer(headers) {
        Ok(token) => token,
        Err(error) => {
            agents::denial(app, "", "gateway.authentication").await?;
            return Err(error);
        }
    };
    let mut db = app.db.pool.acquire().await?;
    match agents::resolve(app, &mut db, &token).await {
        Ok(_) => Ok(token),
        Err(e) => {
            drop(db);
            agents::denial(app, &token, "gateway.authentication").await?;
            Err(e.into())
        }
    }
}
pub async fn human(
    app: &App,
    headers: &HeaderMap,
    write: bool,
    admin: bool,
) -> Result<Human, ApiError> {
    let mut db = app.db.pool.acquire().await?;
    let actor = identity::authenticate(app, &mut db, &cookie(headers)).await?;
    if write {
        let csrf = headers
            .get("x-csrf-token")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        if csrf.is_empty() || !control_core::security::keys::equal(csrf, &actor.csrf) {
            return Err(error("csrf_required", 403));
        }
    }
    if admin && actor.role != "admin" {
        return Err(error("administrator_required", 403));
    }
    Ok(actor)
}
pub async fn login(
    State(app): State<Arc<App>>,
    peer: Option<axum::extract::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    input: Result<Json<Login>, axum::extract::rejection::JsonRejection>,
) -> ApiResult {
    let Json(input) = input?;
    if headers
        .get("x-control-request")
        .and_then(|v| v.to_str().ok())
        != Some("1")
    {
        return Err(error("same_origin_header_required", 403));
    }
    if !regex_username(&input.username)
        || input.password.is_empty()
        || input.password.chars().count() > 256
    {
        return Err(error("invalid_request", 422));
    }
    // Ignore proxy-supplied addresses. Rate limiting uses the real peer in the boundary middleware.
    let (token, actor) = identity::login(
        &app,
        &input.username,
        &input.password,
        &peer
            .map(|p| p.0.0.ip().to_string())
            .unwrap_or_else(|| "unknown-peer".into()),
    )
    .await?;
    let value = format!(
        "ac_session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        app.config.session_seconds,
        if app.config.secure_cookies {
            "; Secure"
        } else {
            ""
        }
    );
    Ok(([(header::SET_COOKIE, value)], Json(actor)).into_response())
}
fn regex_username(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.@-".contains(c))
}
pub async fn me(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    Ok(Json(human(&app, &headers, false, false).await?).into_response())
}
pub async fn logout(State(app): State<Arc<App>>, headers: HeaderMap) -> ApiResult {
    let actor = human(&app, &headers, true, false).await?;
    identity::logout(&app, &cookie(&headers), &actor).await?;
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::SET_COOKIE,
            "ac_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        )],
    )
        .into_response())
}
