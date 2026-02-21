use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Claims extracted from a WorkOS JWT access token.
#[derive(Debug, Clone, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub sid: Option<String>,
    pub org_id: Option<String>,
    pub role: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub exp: usize,
    pub iat: usize,
}

/// Authenticated user context injected into request extensions by auth middleware.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub workos_user_id: String,
    pub session_id: Option<String>,
    pub org_id: Option<String>,
    pub role: Option<String>,
    pub permissions: Option<Vec<String>>,
}

// -- WorkOS API types --

/// User object returned by the WorkOS API.
#[derive(Debug, Deserialize)]
pub struct WorkOsUser {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_verified: Option<bool>,
    pub profile_picture_url: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Response from WorkOS authenticate endpoint.
#[derive(Debug, Deserialize)]
pub struct WorkOsAuthResponse {
    pub user: WorkOsUser,
    pub access_token: String,
    pub refresh_token: String,
}

/// Response from WorkOS create user endpoint.
#[derive(Debug, Deserialize)]
pub struct WorkOsCreateUserResponse {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_verified: Option<bool>,
    pub profile_picture_url: Option<String>,
}

// -- Request/Response DTOs --

/// Request body for user signup.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignupRequest {
    /// User's email address
    #[schema(example = "john@example.com")]
    pub email: String,
    /// Password (min 8 characters)
    #[schema(example = "SecurePass123!")]
    pub password: String,
    #[schema(example = "John")]
    pub first_name: Option<String>,
    #[schema(example = "Doe")]
    pub last_name: Option<String>,
}

/// Request body for email verification.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    /// The 6-digit verification code sent to the user's email
    #[schema(example = "123456")]
    pub code: String,
    /// The pending authentication token returned from signup
    #[schema(example = "JEINf3ozYj2soOa2xi2xzaEIS")]
    pub pending_authentication_token: String,
}

/// Response returned when email verification is required after signup.
#[derive(Debug, Serialize, ToSchema)]
pub struct SignupResponse {
    /// Human-readable message
    pub message: String,
    /// The pending authentication token (pass this to /verify-email)
    pub pending_authentication_token: String,
}

/// WorkOS email verification required error response (parsed internally).
#[derive(Debug, Deserialize)]
pub struct WorkOsEmailVerificationRequired {
    pub code: String,
    pub pending_authentication_token: String,
    pub email_verification_id: String,
}

/// Query parameters for the OAuth authorize endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorizeRequest {
    /// OAuth provider (e.g. "GoogleOAuth", "MicrosoftOAuth", "GitHubOAuth", "AppleOAuth")
    #[schema(example = "GoogleOAuth")]
    pub provider: Option<String>,
    /// WorkOS connection ID for enterprise SSO
    pub connection_id: Option<String>,
    /// WorkOS organization ID
    pub organization_id: Option<String>,
}

/// Response containing the authorization URL.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthorizeUrlResponse {
    /// The URL to redirect the user to for OAuth authorization
    pub authorization_url: String,
}

/// Query parameters received on the OAuth callback.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    /// Authorization code from WorkOS
    pub code: String,
    /// Optional state parameter for CSRF validation
    pub state: Option<String>,
}

/// Request body for user login.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "john@example.com")]
    pub email: String,
    #[schema(example = "SecurePass123!")]
    pub password: String,
}

/// Response returned after successful signup or login.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub user: crate::models::user::UserResponse,
    pub message: String,
    /// Access token (JWT). **DEV ONLY** — remove in production.
    /// Tokens are also set as HttpOnly cookies automatically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
}

/// Generic message response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

/// Error response body.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

/// Error detail within an error response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Error type code (e.g. "UNAUTHORIZED", "NOT_FOUND")
    pub r#type: String,
    /// Human-readable error message
    pub message: String,
}

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
