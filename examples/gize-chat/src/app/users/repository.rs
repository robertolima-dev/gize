use sqlx::SqlitePool;

use super::dto::{CreateUser, UpdateUser};
use super::model::User;

pub async fn list(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
}

pub async fn find(pool: &SqlitePool, id: uuid::Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create(pool: &SqlitePool, input: &CreateUser) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, name, email, password, is_admin) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING *",
    )
        .bind(uuid::Uuid::new_v4())
        .bind(input.name.clone())
        .bind(input.email.clone())
        .bind(input.password.clone())
        .bind(input.is_admin)
        .fetch_one(pool)
        .await
}

pub async fn update(
    pool: &SqlitePool,
    id: uuid::Uuid,
    input: &UpdateUser,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "UPDATE users SET name = ?1, email = ?2, password = ?3, is_admin = ?4, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?5 RETURNING *",
    )
        .bind(input.name.clone())
        .bind(input.email.clone())
        .bind(input.password.clone())
        .bind(input.is_admin)
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn delete(pool: &SqlitePool, id: uuid::Uuid) -> Result<(), sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}
