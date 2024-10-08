use actix_web::{web, HttpResponse, Responder};
use crate::services::auth_service::{create_jwt, create_refresh_token, verify_password, verify_refresh_token};
use crate::services::user_service::get_user_by_email;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    refresh_token: String,
}

// Handler for user login (generates both access and refresh tokens)
pub async fn login_handler(
    pool: web::Data<sqlx::PgPool>,  // Ensure the database connection pool is passed here
    credentials: web::Json<LoginRequest>,
) -> impl Responder {
    // Get user from database by email
    if let Some(user) = get_user_by_email(&pool, &credentials.email).await.unwrap() {
        // Verify the password
        if verify_password(&credentials.password, &user.password_hash).unwrap() {
            // Create JWT access token
            let access_token = create_jwt(user.id);

            // Create refresh token, pass the pool as an argument
            let refresh_token = create_refresh_token(&pool, user.id).await.unwrap();

            // Return the tokens
            return HttpResponse::Ok().json(serde_json::json!({
                "access_token": access_token,
                "refresh_token": refresh_token
            }));
        }
    }

    HttpResponse::Unauthorized().body("Invalid credentials")
}

pub async fn logout_handler(
    pool: web::Data<sqlx::PgPool>,
    request: web::Json<LogoutRequest>,
) -> impl Responder {
    let refresh_token = &request.refresh_token;

    // Mark the refresh token as revoked
    let result = sqlx::query!(
        r#"
        UPDATE refresh_tokens
        SET revoked = TRUE
        WHERE token = $1
        "#,
        refresh_token
    )
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json("Logged out successfully"),
        Err(_) => HttpResponse::InternalServerError().json("Failed to revoke token"),
    }
}

// Handler to refresh access token using a refresh token
pub async fn refresh_token_handler(
    pool: web::Data<sqlx::PgPool>,  // Ensure pool is passed here
    request: web::Json<RefreshTokenRequest>,
) -> impl Responder {
    // Call verify_refresh_token with the pool and await the result
    match verify_refresh_token(pool.get_ref(), &request.refresh_token).await {
        Ok(user_id) => {
            let access_token = create_jwt(user_id);  // Generate new access token
            HttpResponse::Ok().json(serde_json::json!({
                "access_token": access_token
            }))
        }
        Err(_) => HttpResponse::Unauthorized().body("Invalid or revoked refresh token"),
    }
}
