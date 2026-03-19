use sqlx::PgPool;
use uuid::Uuid;

/// Generate a unique test email that won't collide.
pub fn unique_email() -> String {
    format!("test-{}@example.com", Uuid::new_v4())
}

/// Generate a unique WorkOS-style user ID.
pub fn unique_workos_id() -> String {
    format!("user_{}", Uuid::new_v4().simple())
}

/// Generate a unique WorkOS-style org ID.
pub fn unique_workos_org_id() -> String {
    format!("org_{}", Uuid::new_v4().simple())
}

/// Generate a unique mock token (for access/refresh tokens in mock responses).
pub fn unique_token(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

/// Seed a user directly in the test database. Returns the user's internal UUID.
pub async fn seed_user(pool: &PgPool, workos_user_id: &str, email: &str) -> Uuid {
    let row = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (workos_user_id, email, email_verified, role)
        VALUES ($1, $2, true, 'user')
        RETURNING id
        "#,
    )
    .bind(workos_user_id)
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("Failed to seed user (workos_id={workos_user_id}, email={email}): {e}"));

    row
}

/// Seed a user and link them to an organization. Returns (user_id, org_id).
pub async fn seed_user_with_org(
    pool: &PgPool,
    workos_user_id: &str,
    email: &str,
    org_name: &str,
    org_slug: &str,
    workos_org_id: &str,
    role: &str,
) -> (Uuid, Uuid) {
    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (workos_org_id, name, slug)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(workos_org_id)
    .bind(org_name)
    .bind(org_slug)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("Failed to seed organization (slug={org_slug}): {e}"));

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (workos_user_id, email, email_verified, org_id, role)
        VALUES ($1, $2, true, $3, $4)
        RETURNING id
        "#,
    )
    .bind(workos_user_id)
    .bind(email)
    .bind(org_id)
    .bind(role)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("Failed to seed user with org (email={email}): {e}"));

    (user_id, org_id)
}

/// Seed a refresh token for a user. Returns the raw token.
pub async fn seed_refresh_token(pool: &PgPool, user_id: Uuid) -> String {
    let raw_token = format!("test_refresh_{}", Uuid::new_v4());
    let hash = sha2_hash(&raw_token);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query("INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&hash)
        .bind(expires_at)
        .execute(pool)
        .await
        .expect("Failed to seed refresh token");

    raw_token
}

fn sha2_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
