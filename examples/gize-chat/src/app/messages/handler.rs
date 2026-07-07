use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use validator::Validate;

use super::dto::{CreateMessage, UpdateMessage};
use super::error::Error;
use super::model::Message;
use super::service;
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Message>>, Error> {
    let items = service::list(&state.db).await?;
    Ok(Json(items))
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<Message>, Error> {
    let item = service::find(&state.db, id).await?;
    Ok(Json(item))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateMessage>,
) -> Result<(StatusCode, Json<Message>), Error> {
    input.validate()?;
    let item = service::create(&state.db, &input).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(input): Json<UpdateMessage>,
) -> Result<Json<Message>, Error> {
    input.validate()?;
    let item = service::update(&state.db, id, &input).await?;
    Ok(Json(item))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, Error> {
    service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
