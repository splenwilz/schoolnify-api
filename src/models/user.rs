use uuid::Uuid;
use chrono::{DateTime, Utc};

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
