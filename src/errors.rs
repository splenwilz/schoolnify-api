use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    UrlParse(#[from] url::ParseError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            AppError::ExternalService(msg) => {
                tracing::error!(error = %msg, "External service error");
                (
                    StatusCode::BAD_GATEWAY,
                    "EXTERNAL_SERVICE_ERROR",
                    "An external service error occurred".into(),
                )
            }
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".into(),
                )
            }
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".into(),
                )
            }
            AppError::Anyhow(e) => {
                tracing::error!(error = %e, "Unexpected error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".into(),
                )
            }
            AppError::UrlParse(e) => {
                tracing::error!(error = %e, "URL parse error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".into(),
                )
            }
        };

        let body = json!({
            "error": {
                "type": error_type,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}
