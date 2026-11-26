//! Maps OpenRouter error envelopes to [`ProviderHttpError`].

use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::retry::ProviderHttpError;

/// `{"error": {"code", "message", "metadata"}}`.
#[derive(Debug, Deserialize)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct ErrorBody {
    #[serde(default)]
    pub code: Value,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub metadata: Option<ErrorMetadata>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorMetadata {
    #[serde(default)]
    pub error_type: Option<String>,
    /// Moderation reasons on a 403.
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// The error for a non-2xx response with `body`.
pub fn http_error(status: u16, retry_after: Option<Duration>, body: &str) -> ProviderHttpError {
    let (message, metadata) = match serde_json::from_str::<ErrorEnvelope>(body) {
        Ok(envelope) => (
            envelope.error.message,
            envelope.error.metadata.unwrap_or_default(),
        ),
        Err(_) => (body.trim().to_owned(), ErrorMetadata::default()),
    };
    let mut message = match status {
        402 => format!("the OpenRouter account lacks credits: {message}"),
        _ => format!("OpenRouter: {message}"),
    };
    if !metadata.reasons.is_empty() {
        message.push_str(&format!(
            " (moderation reasons: {})",
            metadata.reasons.join(", ")
        ));
    }
    ProviderHttpError {
        status,
        message,
        error_type: metadata.error_type,
        retry_after,
    }
}

/// The error carried by a 2xx body, as `OpenRouter error (code): message`.
pub fn body_error(error: &ErrorBody) -> anyhow::Error {
    anyhow::anyhow!("OpenRouter error ({}): {}", error.code, error.message)
}

#[cfg(test)]
mod tests {
    use super::http_error;

    #[test]
    fn non_json_body_is_kept_as_the_message() {
        let err = http_error(502, None, "<html>Bad gateway</html>");
        assert_eq!(err.message, "OpenRouter: <html>Bad gateway</html>");
        assert_eq!(err.error_type, None);
    }
}
