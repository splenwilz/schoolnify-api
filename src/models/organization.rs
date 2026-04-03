use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for organizations (schools).
#[derive(Debug, Clone, FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub workos_org_id: String,
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API response DTO for organizations.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Organization> for OrganizationResponse {
    fn from(org: Organization) -> Self {
        Self {
            id: org.id,
            name: org.name,
            slug: org.slug,
            domain: org.domain,
            created_at: org.created_at,
        }
    }
}
