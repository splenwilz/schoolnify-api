use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

use crate::errors::AppError;
use crate::models::auth::{AccessTokenClaims, CurrentUser};
use crate::state::AppState;

/// Middleware that validates the JWT access token and injects `CurrentUser` into request extensions.
pub async fn require_auth(
    State(state): State<AppState>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&state, &jar, &request)?;

    let header = decode_header(&token)
        .map_err(|_| AppError::Unauthorized("Invalid token header".into()))?;

    let kid = header
        .kid
        .ok_or_else(|| AppError::Unauthorized("Token missing kid".into()))?;

    let jwks = state.workos_service.get_jwks().await?;

    let jwk = match jwks.keys.iter().find(|k| k.common.key_id.as_deref() == Some(&kid)) {
        Some(k) => k.clone(),
        None => {
            // Key not found — WorkOS may have rotated keys. Force a fresh fetch and retry once.
            tracing::info!(kid = %kid, "JWK kid not found in cache, forcing JWKS refresh");
            let fresh_jwks = state.workos_service.get_jwks_force_refresh().await?;
            fresh_jwks
                .keys
                .into_iter()
                .find(|k| k.common.key_id.as_deref() == Some(&kid))
                .ok_or_else(|| AppError::Unauthorized("Unknown signing key".into()))?
        }
    };

    let decoding_key = DecodingKey::from_jwk(&jwk)
        .map_err(|_| AppError::Unauthorized("Invalid JWK".into()))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_aud = false;
    let expected_issuer = format!(
        "{}/user_management/{}",
        state.config.workos.api_base_url, state.config.workos.client_id
    );
    validation.set_issuer(&[&expected_issuer]);

    let token_data = decode::<AccessTokenClaims>(&token, &decoding_key, &validation).map_err(
        |e| {
            tracing::warn!(error = %e, "JWT validation failed");
            AppError::Unauthorized("Invalid or expired token".into())
        },
    )?;

    let current_user = CurrentUser {
        workos_user_id: token_data.claims.sub,
        session_id: token_data.claims.sid,
        org_id: token_data.claims.org_id,
        role: token_data.claims.role,
        permissions: token_data.claims.permissions,
    };

    request.extensions_mut().insert(current_user);

    Ok(next.run(request).await)
}

/// Extract token from session cookie or Authorization bearer header.
fn extract_token(state: &AppState, jar: &CookieJar, request: &Request) -> Result<String, AppError> {
    // Try cookie first
    if let Some(cookie) = jar.get(&state.config.auth.session_cookie_name) {
        return Ok(cookie.value().to_string());
    }

    // Fallback: Authorization: Bearer <token>
    if let Some(auth_header) = request.headers().get(AUTHORIZATION) {
        let header_str = auth_header
            .to_str()
            .map_err(|_| AppError::Unauthorized("Invalid Authorization header".into()))?;
        if let Some(token) = header_str.strip_prefix("Bearer ") {
            return Ok(token.to_string());
        }
    }

    // Debug: log what cookies we actually received
    let cookie_header = request.headers().get("cookie").map(|v| v.to_str().unwrap_or("<invalid>"));
    let cookie_names: Vec<_> = jar.iter().map(|c| c.name().to_string()).collect();
    tracing::debug!(
        cookie_header = ?cookie_header,
        parsed_cookies = ?cookie_names,
        expected = %state.config.auth.session_cookie_name,
        "No auth token found"
    );

    Err(AppError::Unauthorized(
        "No authentication token provided".into(),
    ))
}
