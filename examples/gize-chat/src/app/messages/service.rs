use sqlx::SqlitePool;

use super::dto::{CreateMessage, UpdateMessage};
use super::error::Error;
use super::model::Message;
use super::repository;

pub async fn list(pool: &SqlitePool) -> Result<Vec<Message>, Error> {
    repository::list(pool).await.map_err(Error::from)
}

pub async fn find(pool: &SqlitePool, id: uuid::Uuid) -> Result<Message, Error> {
    repository::find(pool, id).await.map_err(Error::from)
}

pub async fn create(pool: &SqlitePool, input: &CreateMessage) -> Result<Message, Error> {
    repository::create(pool, input).await.map_err(Error::from)
}

pub async fn update(
    pool: &SqlitePool,
    id: uuid::Uuid,
    input: &UpdateMessage,
) -> Result<Message, Error> {
    repository::update(pool, id, input)
        .await
        .map_err(Error::from)
}

pub async fn delete(pool: &SqlitePool, id: uuid::Uuid) -> Result<(), Error> {
    repository::delete(pool, id).await.map_err(Error::from)
}
