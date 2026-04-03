use serde::Serialize;
use utoipa::ToSchema;

/// Health check response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall health status
    #[schema(example = "healthy")]
    pub status: String,
    /// API version
    #[schema(example = "0.1.0")]
    pub version: String,
    /// Individual service checks
    pub checks: HealthChecks,
}

/// Individual health checks.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthChecks {
    /// Database connectivity status
    #[schema(example = "up")]
    pub database: String,
}
