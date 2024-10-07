use uuid::Uuid;
use chrono::{DateTime, Utc};

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
    pub created_at: DateTime<Utc>,
    pub is_active: Option<bool>,
}
