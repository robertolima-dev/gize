use sqlx::SqlitePool;

use super::dto::{CreateMessage, UpdateMessage};
use super::model::Message;

pub async fn list(pool: &SqlitePool) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>("SELECT * FROM messages ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn find(pool: &SqlitePool, id: uuid::Uuid) -> Result<Message, sqlx::Error> {
    sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create(pool: &SqlitePool, input: &CreateMessage) -> Result<Message, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        "INSERT INTO messages (id, content, username) VALUES (?1, ?2, ?3) RETURNING *",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(input.content.clone())
    .bind(input.username.clone())
    .fetch_one(pool)
    .await
}

pub async fn update(
    pool: &SqlitePool,
    id: uuid::Uuid,
    input: &UpdateMessage,
) -> Result<Message, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        "UPDATE messages SET content = ?1, username = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?3 RETURNING *",
    )
        .bind(input.content.clone())
        .bind(input.username.clone())
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn delete(pool: &SqlitePool, id: uuid::Uuid) -> Result<(), sqlx::Error> {
    let result = sqlx::query("DELETE FROM messages WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}
