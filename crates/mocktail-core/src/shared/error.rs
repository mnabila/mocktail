//! Error taxonomy mapped centrally to HTTP.

use super::http::{Response, api_status_code};
use crate::shared::ids::SlugError;

/// Domain-level invariant violations (pure, no I/O).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("invalid slug: {0}")]
    Slug(#[from] SlugError),
    #[error("validation error: {0}")]
    Validation(String),
}

/// Application-level errors crossing a use-case / port boundary.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// An infra/repository failure, wrapped at the port boundary and never
    /// leaked verbatim to clients.
    #[error("internal error: {0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn status(&self) -> u16 {
        match self {
            AppError::Domain(_) | AppError::BadRequest(_) => 400,
            AppError::Unauthorized(_) => 401,
            AppError::Forbidden(_) => 403,
            AppError::NotFound(_) => 404,
            AppError::Conflict(_) => 409,
            AppError::Internal(_) => 500,
        }
    }

    /// Public-facing message. Internal errors are deliberately opaque.
    pub fn public_message(&self) -> String {
        match self {
            AppError::Internal(_) => "internal error".to_string(),
            other => other.to_string(),
        }
    }

    /// Render a generic JSON problem body (public mock surface / auth routes).
    pub fn to_response(&self) -> Response {
        let body = serde_json::json!({
            "error": {
                "status": self.status(),
                "message": self.public_message(),
            }
        });
        Response::json(self.status(), &body)
    }

    /// Render an admin/dashboard API error using the shared envelope.
    pub fn to_admin_response(&self) -> Response {
        let status = self.status();
        let body = serde_json::json!({
            "status": {
                "code": api_status_code(status),
                "message": self.public_message(),
            },
            "data": {},
            "metadata": {},
        });
        Response::json(status, &body)
    }
}
