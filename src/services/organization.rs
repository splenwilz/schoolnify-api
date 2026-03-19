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
    /// Automatically appends a numeric suffix if the slug already exists.
    /// Retries on unique constraint violation (race between slug check and insert).
    pub async fn create(
        &self,
        workos_org_id: &str,
        name: &str,
        slug: &str,
        domain: Option<&str>,
    ) -> Result<Organization, AppError> {
        for attempt in 0..3 {
            let unique_slug = self.find_unique_slug(slug).await?;

            match sqlx::query_as::<_, Organization>(
                r#"
                INSERT INTO organizations (workos_org_id, name, slug, domain)
                VALUES ($1, $2, $3, $4)
                RETURNING *
                "#,
            )
            .bind(workos_org_id)
            .bind(name)
            .bind(&unique_slug)
            .bind(domain)
            .fetch_one(&self.pool)
            .await
            {
                Ok(org) => return Ok(org),
                Err(sqlx::Error::Database(ref e)) if e.is_unique_violation() && attempt < 2 => {
                    tracing::warn!(slug = %unique_slug, attempt, "Slug collision on insert, retrying");
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(AppError::Conflict(
            "Could not generate a unique slug for this school name".into(),
        ))
    }

    /// Find a unique slug by appending -2, -3, etc. if the base slug is taken.
    /// Uses a single query to fetch all matching slugs.
    async fn find_unique_slug(&self, base_slug: &str) -> Result<String, AppError> {
        let pattern = format!("{base_slug}-%");
        let taken: Vec<String> = sqlx::query_scalar(
            "SELECT slug FROM organizations WHERE slug = $1 OR slug LIKE $2",
        )
        .bind(base_slug)
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        if !taken.contains(&base_slug.to_string()) {
            return Ok(base_slug.to_string());
        }

        for i in 2..=100 {
            let candidate = format!("{base_slug}-{i}");
            if !taken.contains(&candidate) {
                return Ok(candidate);
            }
        }

        Err(AppError::Conflict(
            "Could not generate a unique slug for this school name".into(),
        ))
    }

    /// Find an organization by internal UUID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Organization>, AppError> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(org)
    }

    /// Find an organization by its URL slug.
    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Organization>, AppError> {
        let org = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations WHERE slug = $1",
        )
        .bind(slug)
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

    /// Count the number of admins in an organization.
    pub async fn count_admins(&self, org_id: Uuid) -> Result<i64, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE org_id = $1 AND role = 'admin'",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Delete an organization and unlink all its users.
    pub async fn delete(&self, org_id: Uuid) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        // Unlink users from the org
        sqlx::query("UPDATE users SET org_id = NULL, role = 'member' WHERE org_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;

        // Delete the org
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::OrganizationService;

    #[test]
    fn test_generate_slug_simple() {
        assert_eq!(
            OrganizationService::generate_slug("Springfield High School"),
            "springfield-high-school"
        );
    }

    #[test]
    fn test_generate_slug_special_characters() {
        assert_eq!(
            OrganizationService::generate_slug("St. Mary's Academy"),
            "st-mary-s-academy"
        );
    }

    #[test]
    fn test_generate_slug_extra_spaces() {
        assert_eq!(
            OrganizationService::generate_slug("  Multiple   Spaces  "),
            "multiple-spaces"
        );
    }

    #[test]
    fn test_generate_slug_numbers() {
        assert_eq!(
            OrganizationService::generate_slug("School #123"),
            "school-123"
        );
    }

    #[test]
    fn test_generate_slug_empty_string() {
        assert_eq!(OrganizationService::generate_slug(""), "");
    }

    #[test]
    fn test_generate_slug_only_special_chars() {
        assert_eq!(OrganizationService::generate_slug("---"), "");
    }
}
