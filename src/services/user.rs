use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::auth::WorkOsUser;
use crate::models::user::User;

pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a user from WorkOS data. Creates on first login, updates on subsequent logins.
    pub async fn upsert_from_workos(&self, workos_user: &WorkOsUser) -> Result<User, AppError> {
        let email_verified = workos_user.email_verified.unwrap_or(false);
        let metadata = workos_user
            .metadata
            .clone()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (workos_user_id, email, first_name, last_name, email_verified, profile_picture_url, workos_metadata, last_sign_in_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (workos_user_id)
            DO UPDATE SET
                email = EXCLUDED.email,
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                email_verified = EXCLUDED.email_verified,
                profile_picture_url = EXCLUDED.profile_picture_url,
                workos_metadata = EXCLUDED.workos_metadata,
                last_sign_in_at = NOW()
            RETURNING *
            "#,
        )
        .bind(&workos_user.id)
        .bind(&workos_user.email)
        .bind(&workos_user.first_name)
        .bind(&workos_user.last_name)
        .bind(email_verified)
        .bind(&workos_user.profile_picture_url)
        .bind(&metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Find a user by their internal UUID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(user)
    }

    /// Find a user by their WorkOS user ID.
    pub async fn find_by_workos_id(&self, workos_user_id: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE workos_user_id = $1 AND is_active = TRUE",
        )
        .bind(workos_user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Set the organization for a user.
    pub async fn set_user_org(&self, user_id: Uuid, org_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET org_id = $2 WHERE id = $1")
            .bind(user_id)
            .bind(org_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Set the role for a user.
    pub async fn set_user_role(&self, user_id: Uuid, role: &str) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET role = $2 WHERE id = $1")
            .bind(user_id)
            .bind(role)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a user and all their refresh tokens.
    pub async fn delete_user(&self, user_id: Uuid) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Store a hashed refresh token for a user.
    pub async fn store_refresh_token(
        &self,
        user_id: Uuid,
        raw_token: &str,
        expiry_days: i64,
    ) -> Result<(), AppError> {
        let token_hash = hash_token(raw_token);
        let expires_at = Utc::now() + chrono::Duration::days(expiry_days);

        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Validate that a refresh token exists, is not revoked, and is not expired.
    pub async fn validate_refresh_token(&self, raw_token: &str) -> Result<Uuid, AppError> {
        let token_hash = hash_token(raw_token);

        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT user_id FROM refresh_tokens WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > NOW()",
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((user_id,)) => Ok(user_id),
            None => Err(AppError::Unauthorized(
                "Invalid or expired refresh token".into(),
            )),
        }
    }

    /// Revoke a refresh token (used during logout).
    pub async fn revoke_refresh_token(&self, raw_token: &str) -> Result<(), AppError> {
        let token_hash = hash_token(raw_token);

        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(&token_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Rotate: revoke old token and store new one atomically.
    /// If WorkOS returns the same token, just update the expiry instead.
    pub async fn rotate_refresh_token(
        &self,
        old_raw: &str,
        new_raw: &str,
        expiry_days: i64,
    ) -> Result<(), AppError> {
        // Same token returned — just extend the expiry
        if old_raw == new_raw {
            let token_hash = hash_token(old_raw);
            let expires_at = Utc::now() + chrono::Duration::days(expiry_days);
            sqlx::query(
                "UPDATE refresh_tokens SET expires_at = $2 WHERE token_hash = $1 AND revoked_at IS NULL",
            )
            .bind(&token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        let old_hash = hash_token(old_raw);
        let row = sqlx::query_as::<_, (Uuid,)>(
            "UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > NOW() RETURNING user_id",
        )
        .bind(&old_hash)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Refresh token not found".into()))?;

        let new_hash = hash_token(new_raw);
        let expires_at = Utc::now() + chrono::Duration::days(expiry_days);

        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(row.0)
        .bind(&new_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
