pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;
pub mod state;

use std::time::Duration;

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Schoolnify API",
        description = "REST API for the Schoolnify school management platform",
        version = "0.1.0",
        contact(name = "Schoolnify", url = "https://schoolnify.com"),
    ),
    paths(
        handlers::health::check,
        handlers::auth::signup,
        handlers::auth::verify_email,
        handlers::auth::login,
        handlers::auth::logout,
        handlers::auth::me,
        handlers::auth::refresh,
        handlers::auth::authorize,
        handlers::auth::callback,
        handlers::auth::admin_signup,
        handlers::auth::create_organization,
    ),
    components(schemas(
        models::user::UserResponse,
        models::auth::SignupRequest,
        models::auth::VerifyEmailRequest,
        models::auth::LoginRequest,
        models::auth::AuthResponse,
        models::auth::SignupResponse,
        models::auth::AdminSignupRequest,
        models::auth::AdminSignupResponse,
        models::auth::AdminSignupPendingResponse,
        models::auth::CreateOrganizationRequest,
        models::auth::AuthorizeUrlResponse,
        models::organization::OrganizationResponse,
        models::auth::MessageResponse,
        models::auth::ErrorResponse,
        models::auth::ErrorDetail,
        models::auth::HealthResponse,
        models::auth::HealthChecks,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Auth", description = "Authentication endpoints"),
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_token",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT access token via Authorization header"))
                        .build(),
                ),
            );
            components.add_security_scheme(
                "session_cookie",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .description(Some("Session token via HttpOnly cookie (set automatically on login)"))
                        .build(),
                ),
            );
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    let cors = build_cors_layer(&state);

    let api_routes = routes::build(state.clone())
        .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1 MB
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(state.config.server.request_timeout_secs),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    api_routes.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

fn build_cors_layer(state: &AppState) -> CorsLayer {
    let origins: Vec<HeaderValue> = state
        .config
        .cors
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
}
