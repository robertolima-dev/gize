use sqlx::SqlitePool;

use super::dto::{CreateUser, UpdateUser};
use super::error::Error;
use super::model::User;
use super::repository;

pub async fn list(pool: &SqlitePool) -> Result<Vec<User>, Error> {
    repository::list(pool).await.map_err(Error::from)
}

pub async fn find(pool: &SqlitePool, id: uuid::Uuid) -> Result<User, Error> {
    repository::find(pool, id).await.map_err(Error::from)
}

pub async fn create(pool: &SqlitePool, input: &CreateUser) -> Result<User, Error> {
    repository::create(pool, input).await.map_err(Error::from)
}

pub async fn update(pool: &SqlitePool, id: uuid::Uuid, input: &UpdateUser) -> Result<User, Error> {
    repository::update(pool, id, input)
        .await
        .map_err(Error::from)
}

pub async fn delete(pool: &SqlitePool, id: uuid::Uuid) -> Result<(), Error> {
    repository::delete(pool, id).await.map_err(Error::from)
}
