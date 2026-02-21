use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;

use crate::errors::AppError;
use crate::models::auth::{
    AuthResponse, AuthorizeRequest, AuthorizeUrlResponse, CurrentUser, ErrorResponse, LoginRequest,
    MessageResponse, OAuthCallbackParams, SignupRequest, SignupResponse, VerifyEmailRequest,
    WorkOsAuthResponse,
};
use crate::models::user::UserResponse;
use crate::state::AppState;

/// Create a new user account
///
/// Registers a new user via WorkOS. If email verification is enabled (default),
/// returns a `pending_authentication_token` that must be used with `/verify-email`.
#[utoipa::path(
    post,
    path = "/api/v1/auth/signup",
    tag = "Auth",
    request_body = SignupRequest,
    responses(
        (status = 201, description = "Account created, email verification required", body = SignupResponse),
        (status = 200, description = "Account created and authenticated (verification disabled)", body = AuthResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<SignupRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .workos_service
        .create_user(
            &payload.email,
            &payload.password,
            payload.first_name.as_deref(),
            payload.last_name.as_deref(),
        )
        .await?;

    let auth_result = state
        .workos_service
        .authenticate_with_password(&payload.email, &payload.password)
        .await?;

    match auth_result {
        Ok(auth_response) => {
            let (jar, user) = complete_authentication(&state, jar, &auth_response).await?;
            let response = build_auth_response(
                &state,
                user,
                "Account created successfully",
                &auth_response.access_token,
            );

            Ok((StatusCode::CREATED, jar, Json(response)).into_response())
        }
        Err(email_verification) => {
            let response = SignupResponse {
                message: "Account created. Please check your email for a verification code."
                    .into(),
                pending_authentication_token: email_verification.pending_authentication_token,
            };

            Ok((StatusCode::CREATED, Json(response)).into_response())
        }
    }
}

/// Verify email with code
///
/// Completes email verification using the code sent to the user's email
/// and the `pending_authentication_token` from signup. Returns the user profile with session cookies.
#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-email",
    tag = "Auth",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified and authenticated", body = AuthResponse),
        (status = 400, description = "Invalid verification code", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn verify_email(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<VerifyEmailRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_response = state
        .workos_service
        .authenticate_with_email_verification(&payload.code, &payload.pending_authentication_token)
        .await?;

    let (jar, user) = complete_authentication(&state, jar, &auth_response).await?;
    let response = build_auth_response(
        &state,
        user,
        "Email verified successfully",
        &auth_response.access_token,
    );

    Ok((StatusCode::OK, jar, Json(response)))
}

/// Log in with email and password
///
/// Authenticates a user via WorkOS and returns the user profile with session cookies.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 403, description = "Email verification required", body = SignupResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_result = state
        .workos_service
        .authenticate_with_password(&payload.email, &payload.password)
        .await?;

    match auth_result {
        Ok(auth_response) => {
            let (jar, user) = complete_authentication(&state, jar, &auth_response).await?;
            let response = build_auth_response(
                &state,
                user,
                "Login successful",
                &auth_response.access_token,
            );

            Ok((StatusCode::OK, jar, Json(response)).into_response())
        }
        Err(email_verification) => {
            let response = SignupResponse {
                message: "Email verification required. Please check your email for a verification code.".into(),
                pending_authentication_token: email_verification.pending_authentication_token,
            };

            Ok((StatusCode::FORBIDDEN, Json(response)).into_response())
        }
    }
}

/// Log out the current user
///
/// Clears session cookies and revokes the refresh token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "Auth",
    responses(
        (status = 200, description = "Logged out successfully", body = MessageResponse),
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if let Some(refresh_cookie) = jar.get(&state.config.auth.refresh_cookie_name) {
        state
            .user_service
            .revoke_refresh_token(refresh_cookie.value())
            .await?;
    }

    let jar = jar
        .remove(Cookie::from(state.config.auth.session_cookie_name.clone()))
        .remove(Cookie::from(state.config.auth.refresh_cookie_name.clone()));

    Ok((
        StatusCode::OK,
        jar,
        Json(MessageResponse {
            message: "Logged out successfully".into(),
        }),
    ))
}

/// Get current user profile
///
/// Returns the authenticated user's profile. Requires a valid session cookie or Bearer token.
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Auth",
    security(("session_cookie" = []), ("bearer_token" = [])),
    responses(
        (status = 200, description = "Current user profile", body = UserResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
    )
)]
pub async fn me(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?;

    match user {
        Some(u) => Ok(Json(UserResponse::from(u))),
        None => Err(AppError::NotFound("User not found".into())),
    }
}

/// Refresh access token
///
/// Uses the refresh token from the HttpOnly cookie to obtain a new access token.
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Auth",
    responses(
        (status = 200, description = "Token refreshed", body = MessageResponse),
        (status = 401, description = "Invalid or expired refresh token", body = ErrorResponse),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let refresh_cookie = jar
        .get(&state.config.auth.refresh_cookie_name)
        .ok_or_else(|| AppError::Unauthorized("No refresh token provided".into()))?;

    let raw_refresh = refresh_cookie.value().to_string();

    let user_id = state.user_service.validate_refresh_token(&raw_refresh).await?;

    // Ensure the user account is still active
    let user = state
        .user_service
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User not found".into()))?;
    if !user.is_active {
        return Err(AppError::Forbidden("Account is deactivated".into()));
    }

    let new_tokens = state
        .workos_service
        .refresh_access_token(&raw_refresh)
        .await?;

    state
        .user_service
        .rotate_refresh_token(
            &raw_refresh,
            &new_tokens.refresh_token,
            state.config.auth.refresh_token_expiry_days,
        )
        .await?;

    let jar = set_auth_cookies(
        jar,
        &state,
        &new_tokens.access_token,
        &new_tokens.refresh_token,
    );

    Ok((
        StatusCode::OK,
        jar,
        Json(MessageResponse {
            message: "Token refreshed".into(),
        }),
    ))
}

/// Get OAuth authorization URL
///
/// Returns a WorkOS authorization URL for OAuth/SSO login.
/// The frontend should redirect the user to this URL.
#[utoipa::path(
    get,
    path = "/api/v1/auth/authorize",
    tag = "Auth",
    params(
        ("provider" = Option<String>, Query, description = "OAuth provider (GoogleOAuth, MicrosoftOAuth, GitHubOAuth, AppleOAuth)"),
        ("connection_id" = Option<String>, Query, description = "WorkOS connection ID for enterprise SSO"),
        ("organization_id" = Option<String>, Query, description = "WorkOS organization ID"),
    ),
    responses(
        (status = 200, description = "Authorization URL generated", body = AuthorizeUrlResponse),
        (status = 500, description = "Failed to generate URL", body = ErrorResponse),
    )
)]
pub async fn authorize(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<AuthorizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let authorization_url = state.workos_service.get_authorization_url(
        params.provider.as_deref(),
        params.connection_id.as_deref(),
        params.organization_id.as_deref(),
        None,
    )?;

    Ok(Json(AuthorizeUrlResponse { authorization_url }))
}

/// OAuth callback
///
/// Handles the redirect from WorkOS after OAuth/SSO authentication.
/// Exchanges the authorization code for tokens, upserts the user,
/// sets session cookies, and redirects to the frontend.
#[utoipa::path(
    get,
    path = "/api/v1/auth/callback",
    tag = "Auth",
    params(
        ("code" = String, Query, description = "Authorization code from WorkOS"),
        ("state" = Option<String>, Query, description = "State parameter for CSRF validation"),
    ),
    responses(
        (status = 302, description = "Redirect to frontend with session cookies set"),
        (status = 400, description = "Invalid authorization code", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn callback(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<OAuthCallbackParams>,
) -> Result<impl IntoResponse, AppError> {
    let auth_response = state
        .workos_service
        .authenticate_with_code(&params.code)
        .await?;

    let (jar, _user) = complete_authentication(&state, jar, &auth_response).await?;

    let redirect_url = state.config.auth.post_login_redirect_url.clone();

    Ok((
        StatusCode::FOUND,
        jar,
        [(axum::http::header::LOCATION, redirect_url)],
    ))
}

/// Upserts the user, stores the refresh token, and sets auth cookies.
/// Shared by signup, verify_email, login, and callback handlers.
async fn complete_authentication(
    state: &AppState,
    jar: CookieJar,
    auth_response: &WorkOsAuthResponse,
) -> Result<(CookieJar, crate::models::user::User), AppError> {
    let user = state
        .user_service
        .upsert_from_workos(&auth_response.user)
        .await?;

    state
        .user_service
        .store_refresh_token(
            user.id,
            &auth_response.refresh_token,
            state.config.auth.refresh_token_expiry_days,
        )
        .await?;

    let jar = set_auth_cookies(
        jar,
        state,
        &auth_response.access_token,
        &auth_response.refresh_token,
    );

    Ok((jar, user))
}

/// Build an AuthResponse, conditionally including the access_token based on config.
fn build_auth_response(
    state: &AppState,
    user: crate::models::user::User,
    message: &str,
    access_token: &str,
) -> AuthResponse {
    let expose = state.config.auth.expose_token_in_response;

    AuthResponse {
        user: UserResponse::from(user),
        message: message.into(),
        // Controlled by auth.expose_token_in_response config — set to false in production
        access_token: if expose {
            Some(access_token.to_string())
        } else {
            None
        },
    }
}

fn set_auth_cookies(
    jar: CookieJar,
    state: &AppState,
    access_token: &str,
    refresh_token: &str,
) -> CookieJar {
    let same_site = match state.config.auth.cookie_same_site.as_str() {
        "strict" => SameSite::Strict,
        "none" => SameSite::None,
        _ => SameSite::Lax,
    };

    let mut session_cookie =
        Cookie::new(state.config.auth.session_cookie_name.clone(), access_token.to_string());
    session_cookie.set_http_only(state.config.auth.cookie_http_only);
    session_cookie.set_secure(state.config.auth.cookie_secure);
    session_cookie.set_same_site(same_site);
    session_cookie.set_path("/");
    session_cookie.set_max_age(time::Duration::seconds(
        state.config.auth.access_token_expiry_secs as i64,
    ));
    if !state.config.auth.cookie_domain.is_empty() {
        session_cookie.set_domain(state.config.auth.cookie_domain.clone());
    }

    let mut refresh_cookie =
        Cookie::new(state.config.auth.refresh_cookie_name.clone(), refresh_token.to_string());
    refresh_cookie.set_http_only(true);
    refresh_cookie.set_secure(state.config.auth.cookie_secure);
    refresh_cookie.set_same_site(SameSite::Strict);
    refresh_cookie.set_path("/api/v1/auth/refresh");
    refresh_cookie.set_max_age(time::Duration::days(
        state.config.auth.refresh_token_expiry_days,
    ));
    if !state.config.auth.cookie_domain.is_empty() {
        refresh_cookie.set_domain(state.config.auth.cookie_domain.clone());
    }

    jar.add(session_cookie).add(refresh_cookie)
}
