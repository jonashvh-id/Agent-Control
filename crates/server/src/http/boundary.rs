use super::{auth, response::error};
use axum::{
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use control_core::{App, agents};
use std::sync::Arc;
pub async fn guard(State(app): State<Arc<App>>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let mcp = path == "/mcp/" || path == "/mcp";
    let expected = match crate::config::validate_origin(&app.config) {
        Ok(u) => u,
        Err(_) => return error("invalid_origin", 503).into_response(),
    };
    if request.headers().get_all("host").iter().count() != 1 {
        return error("host_rejected", 400).into_response();
    }
    let authority = request
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if authority != &expected[url::Position::BeforeHost..url::Position::AfterPort] {
        return error("host_rejected", 400).into_response();
    }
    if request.headers().get_all("origin").iter().count() > 1 {
        return error("origin_rejected", 403).into_response();
    }
    if let Some(origin) = request.headers().get("origin")
        && origin.to_str().ok() != Some(&app.config.origin)
    {
        return error("origin_rejected", 403).into_response();
    }
    if mcp {
        let token = match auth::bearer(request.headers()) {
            Ok(t) => t,
            Err(e) => {
                let _ = agents::denial(&app, "", "mcp.authentication").await;
                return e.into_response();
            }
        };
        let mut db = match app.db.pool.acquire().await {
            Ok(db) => db,
            Err(_) => return error("storage_unavailable", 503).into_response(),
        };
        if let Err(e) = agents::resolve(&app, &mut db, &token).await {
            drop(db);
            let _ = agents::denial(&app, &token, "mcp.authentication").await;
            let mut r = super::response::ApiError(e).into_response();
            r.headers_mut()
                .insert("www-authenticate", HeaderValue::from_static("Bearer"));
            return r;
        }
    }
    let (parts, body) = request.into_parts();
    let bytes =
        match tokio::time::timeout(std::time::Duration::from_secs(10), to_bytes(body, 32768)).await
        {
            Ok(Ok(b)) => b,
            Ok(Err(_)) => {
                return error("request_too_large", 413).into_response();
            }
            Err(_) => return error("request_timeout", 408).into_response(),
        };
    if mcp && !bytes.is_empty() && serde_json::from_slice::<serde_json::Value>(&bytes).is_err() {
        return (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Invalid JSON"}}))).into_response();
    }
    let mut response = next
        .run(Request::from_parts(parts, Body::from(bytes)))
        .await;
    if mcp && response.status().is_client_error() {
        response = (
            response.status(),
            axum::Json(serde_json::json!({"error":"invalid_mcp_request"})),
        )
            .into_response();
    }
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
}
