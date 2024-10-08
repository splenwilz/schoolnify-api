use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, TokenData};
use bcrypt::{verify, BcryptError};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use sqlx::types::chrono::DateTime;
use std::env;
use dotenv::dotenv;

const ACCESS_TOKEN_LIFETIME: u64 = 60 * 60; // 1 hour in seconds

// Claims structure for the access token
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

// Claims structure for the refresh token
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: Uuid,
    pub exp: usize,
}

pub fn create_jwt(user_id: Uuid) -> String {
    // Load environment variables
    dotenv().ok();  // Load the .env file

    // Fetch the access token secret from the environment
    let access_token_secret = env::var("ACCESS_TOKEN_SECRET")
        .expect("ACCESS_TOKEN_SECRET must be set in .env file");

    // Calculate expiration time (1 hour from now)
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() + ACCESS_TOKEN_LIFETIME;

    let claims = Claims {
        sub: user_id,
        exp: expiration as usize,
    };

    // Create JWT using the secret loaded from the environment
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(access_token_secret.as_bytes()),
    ).expect("Failed to create JWT")
}

pub async fn create_refresh_token(pool: &PgPool, user_id: Uuid) -> Result<String, sqlx::Error> {
    // Load environment variables
    dotenv().ok();  // Load the .env file

    // Fetch the access token secret from the environment
    let refresh_token_secret = env::var("REFRESH_TOKEN_SECRET")
        .expect("REFRESH_TOKEN_SECRET must be set in .env file");


    let expiration = Utc::now() + Duration::days(30); // 30-day expiration

    let claims = RefreshTokenClaims {
        sub: user_id,
        exp: expiration.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(refresh_token_secret.as_bytes()),
    ).expect("Failed to create refresh token");

    // Store refresh token in the database
    sqlx::query!(
        r#"
        INSERT INTO refresh_tokens (user_id, token, expires_at)
        VALUES ($1, $2, $3)
        "#,
        user_id,
        token,
        DateTime::<Utc>::from_naive_utc_and_offset(expiration.naive_utc(), Utc)
    )
    .execute(pool)
    .await?;

    Ok(token)
}


// Verify JWT (access token)
pub fn verify_jwt(token: &str) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    // Load environment variables
    dotenv().ok();  // Load the .env file

    // Fetch the access token secret from the environment
    let access_token_secret = env::var("ACCESS_TOKEN_SECRET")
        .expect("ACCESS_TOKEN_SECRET must be set in .env file");
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(access_token_secret.as_bytes()),
        &Validation::default(),
    )
}

pub async fn verify_refresh_token(pool: &PgPool, token: &str) -> Result<Uuid, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT user_id, revoked
        FROM refresh_tokens
        WHERE token = $1
        "#,
        token
    )
    .fetch_one(pool)
    .await?;

    // Use `unwrap_or(false)` to handle `NULL` values as `false`
    if result.revoked.unwrap_or(false) {
        return Err(sqlx::Error::RowNotFound);  // Treat revoked token as invalid
    }

    Ok(result.user_id)
}

// Verify password using bcrypt
pub fn verify_password(plain_password: &str, hashed_password: &str) -> Result<bool, BcryptError> {
    verify(plain_password, hashed_password)
}
