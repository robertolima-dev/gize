use serde::{Deserialize, Serialize};

/// The `Message` domain model, mapped to the `messages` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: uuid::Uuid,
    pub content: String,
    pub username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
