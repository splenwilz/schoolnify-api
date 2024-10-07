// use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(serde::Serialize)] 
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub domain: Option<String>,
    pub address: String,
    pub contact_email: String,
    pub contact_phone: Option<String>,
    pub logo_url: Option<String>,
    pub timezone: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,  // Change to DateTime<Utc>
    pub is_active: Option<bool>,
}

#[derive(serde::Serialize)] 
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: Option<chrono::NaiveDate>, // Nullable
    pub gender: Option<String>,
    pub profile_picture_url: Option<String>,
    pub contact_phone: Option<String>,
    pub address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}



#[derive(Deserialize, ToSchema)]
pub struct TenantRequest {
    pub name: String,
    pub domain: Option<String>,
    pub address: String,
    pub contact_email: String,
    pub contact_phone: Option<String>,
    pub logo_url: Option<String>,
    pub timezone: String,
}

#[derive(Serialize, ToSchema)]
pub struct TenantResponse {
    pub id: uuid::Uuid,
    pub message: String,
}

