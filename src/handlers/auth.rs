use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;

use crate::errors::AppError;
use crate::models::auth::{
    AdminSignupPendingResponse, AdminSignupRequest, AdminSignupResponse, AuthResponse,
    AuthorizeRequest, AuthorizeUrlResponse, CreateOrganizationRequest, CurrentUser, ErrorResponse,
    LoginRequest, MessageResponse, OAuthCallbackParams, ResendVerificationRequest, SignupRequest,
    SignupResponse, VerifyEmailRequest, WorkOsAuthResponse,
};
use crate::models::organization::OrganizationResponse;
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
    let created_user = state
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
                &auth_response.refresh_token,
            );

            Ok((StatusCode::CREATED, jar, Json(response)).into_response())
        }
        Err(email_verification) => {
            let response = SignupResponse {
                message: "Account created. Please check your email for a verification code."
                    .into(),
                pending_authentication_token: email_verification.pending_authentication_token,
                user_id: Some(created_user.id),
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
        &auth_response.refresh_token,
    );

    Ok((StatusCode::OK, jar, Json(response)))
}

/// Resend email verification code
///
/// Sends a new verification code to the user's email. Requires the WorkOS user ID
/// returned from the signup response.
#[utoipa::path(
    post,
    path = "/api/v1/auth/resend-verification",
    tag = "Auth",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Verification email sent", body = MessageResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn resend_verification(
    State(state): State<AppState>,
    Json(payload): Json<ResendVerificationRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .workos_service
        .send_verification_email(&payload.user_id)
        .await?;

    Ok((
        StatusCode::OK,
        Json(MessageResponse {
            message: "Verification email sent".into(),
        }),
    ))
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
                &auth_response.refresh_token,
            );

            Ok((StatusCode::OK, jar, Json(response)).into_response())
        }
        Err(email_verification) => {
            let response = SignupResponse {
                message: "Email verification required. Please check your email for a verification code.".into(),
                pending_authentication_token: email_verification.pending_authentication_token,
                user_id: None,
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

    let mut session_removal = Cookie::new(state.config.auth.session_cookie_name.clone(), "");
    session_removal.set_path("/");
    if !state.config.auth.cookie_domain.is_empty() {
        session_removal.set_domain(state.config.auth.cookie_domain.clone());
    }

    let mut refresh_removal = Cookie::new(state.config.auth.refresh_cookie_name.clone(), "");
    refresh_removal.set_path("/api/v1/auth");
    if !state.config.auth.cookie_domain.is_empty() {
        refresh_removal.set_domain(state.config.auth.cookie_domain.clone());
    }

    let jar = jar.remove(session_removal).remove(refresh_removal);

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

/// Delete current user account
///
/// Permanently deletes the authenticated user's account from both WorkOS and the local database.
/// Clears session cookies and revokes all refresh tokens.
#[utoipa::path(
    delete,
    path = "/api/v1/auth/me",
    tag = "Auth",
    security(("session_cookie" = []), ("bearer_token" = [])),
    responses(
        (status = 200, description = "Account deleted successfully", body = MessageResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn delete_account(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    // If user is the sole admin of an org, delete the org too
    if let Some(org_id) = user.org_id
        && user.role == "admin"
    {
        let admin_count = state.organization_service.count_admins(org_id).await?;
        if admin_count <= 1 {
            // Last admin — delete the org from WorkOS and locally
            if let Some(org) = state.organization_service.find_by_id(org_id).await? {
                state
                    .workos_service
                    .delete_organization(&org.workos_org_id)
                    .await?;
            }
            state.organization_service.delete(org_id).await?;
        }
    }

    // Delete user from WorkOS
    state
        .workos_service
        .delete_user(&current_user.workos_user_id)
        .await?;

    // Delete locally (user + refresh tokens)
    state.user_service.delete_user(user.id).await?;

    // Clear cookies
    let mut session_removal = Cookie::new(state.config.auth.session_cookie_name.clone(), "");
    session_removal.set_path("/");
    if !state.config.auth.cookie_domain.is_empty() {
        session_removal.set_domain(state.config.auth.cookie_domain.clone());
    }

    let mut refresh_removal = Cookie::new(state.config.auth.refresh_cookie_name.clone(), "");
    refresh_removal.set_path("/api/v1/auth");
    if !state.config.auth.cookie_domain.is_empty() {
        refresh_removal.set_domain(state.config.auth.cookie_domain.clone());
    }

    let jar = jar.remove(session_removal).remove(refresh_removal);

    Ok((
        StatusCode::OK,
        jar,
        Json(MessageResponse {
            message: "Account deleted successfully".into(),
        }),
    ))
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
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<AuthorizeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let nonce = uuid::Uuid::new_v4().to_string();

    let authorization_url = state.workos_service.get_authorization_url(
        params.provider.as_deref(),
        params.connection_id.as_deref(),
        params.organization_id.as_deref(),
        Some(&nonce),
    )?;

    let mut nonce_cookie = Cookie::new("oauth_state", nonce);
    nonce_cookie.set_http_only(true);
    nonce_cookie.set_secure(state.config.auth.cookie_secure);
    nonce_cookie.set_same_site(SameSite::Lax);
    nonce_cookie.set_path("/api/v1/auth/callback");
    nonce_cookie.set_max_age(time::Duration::minutes(10));

    let jar = jar.add(nonce_cookie);

    Ok((jar, Json(AuthorizeUrlResponse { authorization_url })))
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
    // Validate CSRF state parameter
    let expected_state = jar
        .get("oauth_state")
        .ok_or_else(|| AppError::BadRequest("Missing OAuth state cookie".into()))?;

    match &params.state {
        Some(state_param) if state_param == expected_state.value() => {}
        _ => return Err(AppError::BadRequest("Invalid OAuth state parameter".into())),
    }

    let auth_response = state
        .workos_service
        .authenticate_with_code(&params.code)
        .await?;

    let (jar, _user) = complete_authentication(&state, jar, &auth_response).await?;

    // Clear the nonce cookie
    let mut nonce_removal = Cookie::new("oauth_state", "");
    nonce_removal.set_path("/api/v1/auth/callback");
    let jar = jar.remove(nonce_removal);

    let redirect_url = state.config.auth.post_login_redirect_url.clone();

    Ok((
        StatusCode::FOUND,
        jar,
        [(axum::http::header::LOCATION, redirect_url)],
    ))
}

/// Register as a school admin
///
/// Creates a new user and school organization. The user becomes the admin of the school.
/// If email verification is required, returns a pending token — call `/verify-email` first,
/// then `POST /api/v1/organizations` to complete school setup.
#[utoipa::path(
    post,
    path = "/api/v1/auth/admin-signup",
    tag = "Auth",
    request_body = AdminSignupRequest,
    responses(
        (status = 201, description = "Admin account and school created", body = AdminSignupResponse),
        (status = 202, description = "Email verification required before school creation", body = AdminSignupPendingResponse),
        (status = 409, description = "Email already exists", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn admin_signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<AdminSignupRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Step 1: Create user in WorkOS
    let created_user = state
        .workos_service
        .create_user(
            &payload.email,
            &payload.password,
            payload.first_name.as_deref(),
            payload.last_name.as_deref(),
        )
        .await?;

    // Step 2: Authenticate
    let auth_result = state
        .workos_service
        .authenticate_with_password(&payload.email, &payload.password)
        .await?;

    match auth_result {
        Ok(auth_response) => {
            // Step 3: Upsert local user
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

            // Step 4-8: Create org and assign admin role
            let org = setup_organization(
                &state,
                &auth_response.user.id,
                user.id,
                &payload.school_name,
            )
            .await?;

            // Step 9: Refresh token with org context to get org_id in JWT
            let new_tokens = state
                .workos_service
                .refresh_access_token_with_org(
                    &auth_response.refresh_token,
                    &org.workos_org_id,
                )
                .await?;

            // Step 10: Rotate refresh token
            state
                .user_service
                .rotate_refresh_token(
                    &auth_response.refresh_token,
                    &new_tokens.refresh_token,
                    state.config.auth.refresh_token_expiry_days,
                )
                .await?;

            // Step 11: Set cookies and respond
            let jar = set_auth_cookies(
                jar,
                &state,
                &new_tokens.access_token,
                &new_tokens.refresh_token,
            );

            let expose = state.config.auth.expose_token_in_response;
            let subdomain_url = build_subdomain_url(&state, &org.slug);
            let response = AdminSignupResponse {
                user: crate::models::user::UserResponse::from(
                    state.user_service.find_by_id(user.id).await?
                        .ok_or_else(|| AppError::Internal("User not found after upsert".into()))?,
                ),
                organization: OrganizationResponse::from(org),
                message: "School admin account created successfully".into(),
                access_token: if expose {
                    Some(new_tokens.access_token)
                } else {
                    None
                },
                subdomain_url,
            };

            Ok((StatusCode::CREATED, jar, Json(response)).into_response())
        }
        Err(email_verification) => {
            let response = AdminSignupPendingResponse {
                message: "Account created. Verify your email, then complete school setup.".into(),
                pending_authentication_token: email_verification.pending_authentication_token,
                school_name: payload.school_name,
                user_id: created_user.id,
            };

            Ok((StatusCode::ACCEPTED, Json(response)).into_response())
        }
    }
}

/// Create a school organization
///
/// Creates a new school organization for the authenticated user.
/// Used after email verification in the admin signup flow.
/// The user must not already belong to an organization.
#[utoipa::path(
    post,
    path = "/api/v1/auth/create-organization",
    tag = "Auth",
    security(("session_cookie" = []), ("bearer_token" = [])),
    request_body = CreateOrganizationRequest,
    responses(
        (status = 201, description = "School organization created", body = AdminSignupResponse),
        (status = 409, description = "User already belongs to an organization", body = ErrorResponse),
        (status = 502, description = "WorkOS service error", body = ErrorResponse),
    )
)]
pub async fn create_organization(
    Extension(current_user): Extension<CurrentUser>,
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<CreateOrganizationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .user_service
        .find_by_workos_id(&current_user.workos_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if user.org_id.is_some() {
        return Err(AppError::Conflict(
            "User already belongs to an organization".into(),
        ));
    }

    // Create org and assign admin
    let org = setup_organization(
        &state,
        &current_user.workos_user_id,
        user.id,
        &payload.school_name,
    )
    .await?;

    // Prefer refresh_token from request body (avoids cookie timing issues),
    // fall back to cookie.
    let raw_refresh = if let Some(token) = payload.refresh_token {
        token
    } else {
        jar.get(&state.config.auth.refresh_cookie_name)
            .ok_or_else(|| AppError::Unauthorized("No refresh token provided".into()))?
            .value()
            .to_string()
    };

    let new_tokens = state
        .workos_service
        .refresh_access_token_with_org(&raw_refresh, &org.workos_org_id)
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

    let expose = state.config.auth.expose_token_in_response;
    let subdomain_url = build_subdomain_url(&state, &org.slug);
    let response = AdminSignupResponse {
        user: crate::models::user::UserResponse::from(
            state.user_service.find_by_id(user.id).await?
                        .ok_or_else(|| AppError::Internal("User not found after upsert".into()))?,
        ),
        organization: OrganizationResponse::from(org),
        message: "School organization created successfully".into(),
        access_token: if expose {
            Some(new_tokens.access_token)
        } else {
            None
        },
        subdomain_url,
    };

    Ok((StatusCode::CREATED, jar, Json(response)))
}

/// Shared logic: create org in WorkOS + local DB, create membership, link user.
async fn setup_organization(
    state: &AppState,
    workos_user_id: &str,
    local_user_id: uuid::Uuid,
    school_name: &str,
) -> Result<crate::models::organization::Organization, AppError> {
    use crate::services::organization::OrganizationService;

    // Create org in WorkOS
    let workos_org = state
        .workos_service
        .create_organization(school_name, None)
        .await?;

    // Create local org
    let slug = OrganizationService::generate_slug(school_name);
    let org = state
        .organization_service
        .create(&workos_org.id, school_name, &slug, None)
        .await?;

    // Create membership in WorkOS (admin role)
    state
        .workos_service
        .create_organization_membership(workos_user_id, &workos_org.id, "admin")
        .await?;

    // Link user to org locally and set role
    state.user_service.set_user_org(local_user_id, org.id).await?;
    state.user_service.set_user_role(local_user_id, "admin").await?;

    Ok(org)
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
    refresh_token: &str,
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
        refresh_token: if expose {
            Some(refresh_token.to_string())
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
    refresh_cookie.set_same_site(same_site);
    refresh_cookie.set_path("/api/v1/auth");
    refresh_cookie.set_max_age(time::Duration::days(
        state.config.auth.refresh_token_expiry_days,
    ));
    if !state.config.auth.cookie_domain.is_empty() {
        refresh_cookie.set_domain(state.config.auth.cookie_domain.clone());
    }

    tracing::debug!(
        session_name = %session_cookie.name(),
        session_path = ?session_cookie.path(),
        session_domain = ?session_cookie.domain(),
        session_secure = ?session_cookie.secure(),
        session_same_site = ?session_cookie.same_site(),
        session_http_only = ?session_cookie.http_only(),
        "Setting auth cookies"
    );

    jar.add(session_cookie).add(refresh_cookie)
}

/// Build the subdomain URL for a school based on its slug and the configured base domain.
fn build_subdomain_url(state: &AppState, slug: &str) -> String {
    let base_domain = &state.config.cors.base_domain;
    if base_domain == "localhost" {
        format!("http://{slug}.localhost:3001")
    } else {
        format!("https://{slug}.{base_domain}")
    }
}
