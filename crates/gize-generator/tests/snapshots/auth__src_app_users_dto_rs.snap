use serde::{Deserialize, Serialize};
use validator::Validate;

/// Payload to create a `User`. `password` is plaintext on the wire and hashed before storage.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUser {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub password: String,
    pub is_admin: bool,
}

/// Payload to update a `User`.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUser {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub password: String,
    pub is_admin: bool,
}

/// Credentials for `POST /users/login`.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "must not be empty"))]
    pub password: String,
}

/// A signed session token returned by register/login.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
}
