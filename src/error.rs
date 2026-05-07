//! HTTP error mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Upstream(String),
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.as_str()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.as_str()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.as_str()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.as_str()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.as_str()),
            AppError::Unavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m.as_str()),
            AppError::Upstream(m) => (StatusCode::BAD_GATEWAY, m.as_str()),
            AppError::Sql(e) => {
                tracing::error!(%e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "database error")
            }
            AppError::Any(e) => {
                tracing::error!(%e, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        };
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
