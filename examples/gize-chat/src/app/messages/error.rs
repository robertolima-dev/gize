use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Errors returned by the `messages` resource.
#[derive(Debug)]
pub enum Error {
    NotFound,
    /// A unique constraint was violated (e.g. a duplicate key) — maps to 409.
    Conflict,
    /// A foreign-key constraint was violated (a referenced record is missing, or is still
    /// referenced by another row) — maps to 409.
    ForeignKey,
    /// Request payload failed validation — maps to 422.
    Validation(String),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::RowNotFound = error {
            return Error::NotFound;
        }
        // Map database integrity violations to client errors instead of a generic 500.
        if let sqlx::Error::Database(ref db) = error {
            match db.code().as_deref() {
                Some("2067") => return Error::Conflict,  // unique violation
                Some("787") => return Error::ForeignKey, // foreign-key violation
                _ => {}
            }
        }
        Error::Database(error)
    }
}

impl From<validator::ValidationErrors> for Error {
    fn from(errors: validator::ValidationErrors) -> Self {
        Error::Validation(errors.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::Conflict => (StatusCode::CONFLICT, "already exists".to_string()),
            Error::ForeignKey => (
                StatusCode::CONFLICT,
                "a referenced record does not exist or is still in use".to_string(),
            ),
            Error::Validation(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
            Error::Database(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };
        (status, message).into_response()
    }
}
