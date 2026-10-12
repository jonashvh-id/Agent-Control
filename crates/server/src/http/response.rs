use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
pub struct ApiError(pub control_core::error::Error);
pub type ApiResult = Result<Response, ApiError>;
impl From<control_core::error::Error> for ApiError {
    fn from(e: control_core::error::Error) -> Self {
        Self(e)
    }
}
impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        Self(e.into())
    }
}
impl From<JsonRejection> for ApiError {
    fn from(_: JsonRejection) -> Self {
        Self(control_core::error::Error::new("invalid_request", 422))
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.0.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({"error":self.0.code})),
        )
            .into_response()
    }
}
pub fn json(value: serde_json::Value) -> Response {
    Json(value).into_response()
}
pub fn error(code: &'static str, status: u16) -> ApiError {
    ApiError(control_core::error::Error::new(code, status))
}
