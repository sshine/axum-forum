use axum::{
    http::{self, StatusCode},
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::forum::PostId;

pub type ForumResult<T> = Result<T, ForumError>;

#[derive(Debug, Error)]
pub enum ForumError {
    #[error("Database error: {0}")]
    DatabaseError(rusqlite::Error),

    #[error("Template error: {0}")]
    TemplateError(minijinja::Error),

    #[error("Lock poisoned: {0}")]
    LockError(String),

    #[error("HTTP error: {0}")]
    HttpError(http::Error),

    #[error("Validation error: {0}")]
    ValidationError(&'static str),

    #[error("Post with id {0} not found")]
    NotFound(PostId),
}

impl IntoResponse for ForumError {
    fn into_response(self) -> Response {
        let status_code = match &self {
            ForumError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ForumError::TemplateError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ForumError::LockError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ForumError::HttpError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ForumError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ForumError::NotFound(_) => StatusCode::BAD_REQUEST,
        };

        (status_code, format!("{}", self)).into_response()
    }
}
