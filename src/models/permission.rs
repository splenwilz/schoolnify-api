use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub code: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}
