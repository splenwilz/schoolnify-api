use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::organization::Organization;

pub struct OrganizationService {
    pool: PgPool,
}

impl OrganizationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new organization (school) in the local database.
    pub async fn create(
        &self,
        workos_org_id: &str,
        name: &str,
        slug: &str,
        domain: Option<&str>,
    ) -> Result<Organization, AppError> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (workos_org_id, name, slug, domain)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(workos_org_id)
        .bind(name)
        .bind(slug)
        .bind(domain)
        .fetch_one(&self.pool)
        .await?;

        Ok(org)
    }

    /// Find an organization by internal UUID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Organization>, AppError> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(org)
    }

    /// Find an organization by WorkOS organization ID.
    pub async fn find_by_workos_id(
        &self,
        workos_org_id: &str,
    ) -> Result<Option<Organization>, AppError> {
        let org = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE workos_org_id = $1",
        )
        .bind(workos_org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(org)
    }

    /// Generate a URL-friendly slug from a school name.
    /// e.g. "Springfield High School" → "springfield-high-school"
    pub fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}
