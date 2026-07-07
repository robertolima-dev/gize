use serde::Deserialize;
use validator::Validate;

/// Payload to create a `Message`.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateMessage {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub content: String,
    #[validate(length(min = 1, message = "must not be empty"))]
    pub username: String,
}

/// Payload to update a `Message`.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMessage {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub content: String,
    #[validate(length(min = 1, message = "must not be empty"))]
    pub username: String,
}
