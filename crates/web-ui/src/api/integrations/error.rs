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
    /// Seconds for a `Retry-After` header.
    retry_after: Option<u64>,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            retry_after: None,
        }
    }

    /// `429`, telling the client to wait `retry_after` (at least one second).
    pub fn too_many_requests(message: impl Into<String>, retry_after: std::time::Duration) -> Self {
        Self {
            retry_after: Some(retry_after.as_secs_f64().ceil().max(1.0) as u64),
            ..Self::new(StatusCode::TOO_MANY_REQUESTS, message)
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
        let mut response = json_error(self.status, self.message);
        if let Some(seconds) = self.retry_after {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, seconds.into());
        }
        response
    }
}
