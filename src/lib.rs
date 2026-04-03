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
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
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
        handlers::auth::resend_verification,
        handlers::auth::login,
        handlers::auth::logout,
        handlers::auth::me,
        handlers::auth::delete_account,
        handlers::auth::refresh,
        handlers::auth::authorize,
        handlers::auth::callback,
        handlers::auth::admin_signup,
        handlers::auth::create_organization,
        handlers::auth::establish_session,
    ),
    components(schemas(
        models::user::UserResponse,
        models::auth::SignupRequest,
        models::auth::VerifyEmailRequest,
        models::auth::LoginRequest,
        models::auth::AuthResponse,
        models::auth::SignupResponse,
        models::auth::ResendVerificationRequest,
        models::auth::AdminSignupRequest,
        models::auth::AdminSignupResponse,
        models::auth::AdminSignupPendingResponse,
        models::auth::EstablishSessionRequest,
        models::auth::CreateOrganizationRequest,
        models::auth::AuthorizeUrlResponse,
        models::organization::OrganizationResponse,
        models::auth::MessageResponse,
        models::auth::ErrorResponse,
        models::auth::ErrorDetail,
        models::health::HealthResponse,
        models::health::HealthChecks,
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
                SecurityScheme::ApiKey(ApiKey::Cookie(
                    ApiKeyValue::with_description(
                        "session_token",
                        "HttpOnly session cookie (set automatically on login)",
                    ),
                )),
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
    let base_domain = state.config.cors.base_domain.clone();
    let static_origins: Vec<HeaderValue> = state
        .config
        .cors
        .allowed_origins
        .iter()
        .filter_map(|o| match o.parse() {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!(origin = %o, error = %e, "Ignoring invalid CORS origin");
                None
            }
        })
        .collect();

    let allow_origin = AllowOrigin::predicate(move |origin, _parts| {
        // 1. Check static allowlist (localhost dev origins)
        if static_origins.iter().any(|allowed| allowed == origin) {
            return true;
        }

        // 2. Check dynamic subdomain pattern: {slug}.{base_domain}
        let origin_str = match origin.to_str() {
            Ok(s) => s,
            Err(_) => return false,
        };

        if let Ok(url) = url::Url::parse(origin_str)
            && let Some(host) = url.host_str()
        {
            let suffix = format!(".{base_domain}");
            if let Some(slug) = host.strip_suffix(&suffix) {
                return !slug.is_empty()
                        && !slug.contains('.')
                        && slug.chars().any(|c| c.is_ascii_alphanumeric());
            }
            // Allow bare base domain
            if host == base_domain {
                return true;
            }
        }

        false
    });

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
}
