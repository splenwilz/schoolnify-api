use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for the `users` table.
/// Does NOT derive Serialize — always convert to UserResponse for API output.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub workos_user_id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_verified: bool,
    pub profile_picture_url: Option<String>,
    pub workos_metadata: serde_json::Value,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub org_id: Option<Uuid>,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API response representation (omits internal fields).
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    /// Unique user identifier
    pub id: Uuid,
    /// User's email address
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_verified: bool,
    pub profile_picture_url: Option<String>,
    /// Organization (school) this user belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    /// User role (e.g. "user", "admin", "teacher")
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            first_name: u.first_name,
            last_name: u.last_name,
            email_verified: u.email_verified,
            profile_picture_url: u.profile_picture_url,
            organization_id: u.org_id,
            role: u.role,
            created_at: u.created_at,
        }
    }
}
