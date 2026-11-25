//! The error type of the integration handlers.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use assistant_core::integrations::IntegrationStoreError;

use crate::errors::json_error;

/// A failed request: a status and the message of the `{"error": ...}` envelope.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    /// The request is well-formed JSON but names something invalid.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, message)
    }
}

/// `DuplicateSlug` and `Stale` are a `409`, `ConnectUrlImmutable` a `422`; anything else is
/// an internal error.
impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        match error.downcast_ref::<IntegrationStoreError>() {
            Some(
                e @ (IntegrationStoreError::DuplicateSlug { .. }
                | IntegrationStoreError::Stale { .. }),
            ) => Self::new(StatusCode::CONFLICT, e.to_string()),
            Some(e @ IntegrationStoreError::ConnectUrlImmutable { .. }) => {
                Self::unprocessable(e.to_string())
            }
            None => {
                tracing::error!("integrations api: {error:#}");
                Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        }
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        Self::new(rejection.status(), rejection.body_text())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        Self::new(rejection.status(), rejection.body_text())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        json_error(self.status, self.message)
    }
}
