use sqlx::PgPool;
use crate::models::user::User;
use chrono::NaiveDate;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct UserRequest {
    pub email: String,
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
}

// Function to create a new user
pub async fn create_user(pool: &PgPool, request: &UserRequest) -> Result<uuid::Uuid, sqlx::Error> {
    let new_user_id = uuid::Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO "User" (id, email, password_hash, first_name, last_name, date_of_birth, gender, contact_phone, address)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        new_user_id,
        request.email,
        request.password_hash,
        request.first_name,
        request.last_name,
        request.date_of_birth.as_ref().and_then(|dob| NaiveDate::parse_from_str(dob, "%Y-%m-%d").ok()), // Convert String to NaiveDate
        request.gender,
        request.contact_phone,
        request.address
    )
    .execute(pool)
    .await?;

    Ok(new_user_id)
}

// Function to fetch all users
pub async fn get_users(pool: &PgPool) -> Result<Vec<User>, sqlx::Error> {
    let users = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password_hash, first_name, last_name, date_of_birth, gender, contact_phone, address, created_at, last_login_at, is_active, profile_picture_url
        FROM "User"
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

pub async fn update_user(
    pool: &PgPool,
    user_id: Uuid,
    email: Option<&str>,
    first_name: Option<&str>,
    last_name: Option<&str>,
    contact_phone: Option<&str>,
    address: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE "User"
        SET email = COALESCE($2, email),
            first_name = COALESCE($3, first_name),
            last_name = COALESCE($4, last_name),
            contact_phone = COALESCE($5, contact_phone),
            address = COALESCE($6, address)
        WHERE id = $1
        "#,
        user_id,
        email,
        first_name,
        last_name,
        contact_phone,
        address
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM "User"
        WHERE id = $1
        "#,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password_hash, first_name, last_name, date_of_birth, gender, contact_phone, address, created_at, last_login_at, is_active, profile_picture_url
        FROM "User"
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}


pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, email, password_hash, first_name, last_name, date_of_birth, gender, contact_phone, address, created_at, last_login_at, is_active, profile_picture_url
        FROM "User"
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}
