use axum::{
    http::{self, StatusCode},
    response::{IntoResponse, Response},
};
use css_minify::optimizations::MError;
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

    #[error("Unknown post ID: {0}")]
    PostNotFound(PostId),

    #[error("Environment variable error: {0}")]
    EnvVarError(std::env::VarError),

    #[error("Environment variable error: {0}")]
    EnvParseError(String),

    #[error("Config error: {0}")]
    ConfigError(config_manager::Error),

    #[error("CSS error: {0}")]
    CssMinifyError(MError<'static>),
}

impl IntoResponse for ForumError {
    fn into_response(self) -> Response {
        let status_code = match &self {
            ForumError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ForumError::PostNotFound(_) => StatusCode::NOT_FOUND,
            _otherwise => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status_code, format!("{}", self)).into_response()
    }
}
