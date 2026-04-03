use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::models::health::{HealthChecks, HealthResponse};
use crate::state::AppState;

/// Check API health
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse),
    )
)]
pub async fn check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_healthy = match sqlx::query("SELECT 1").execute(&state.db_pool).await {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!(error = %e, "Health check: database unreachable");
            false
        }
    };

    let status = if db_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(HealthResponse {
            status: if db_healthy { "healthy" } else { "unhealthy" }.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            checks: HealthChecks {
                database: if db_healthy { "up" } else { "down" }.into(),
            },
        }),
    )
}
