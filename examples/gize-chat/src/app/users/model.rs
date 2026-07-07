use serde::{Deserialize, Serialize};

/// The `User` domain model, mapped to the `users` table.
///
/// `password` stores a hash and is never serialised into API responses. It is still read
/// from the database (`sqlx::FromRow`) and accepted on create/update through the DTOs.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    // Loaded from the database but not serialised into responses; it is read once
    // authentication (login/password verification) lands. Allowed to be unused so a
    // freshly scaffolded app stays clippy-clean under `-D warnings`.
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub password: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
