//! HTTP action executor for workflow nodes.

use std::time::Duration;

use anyhow::{Context, Result};
use assistant_workflow::{WorkflowActionExecutor, WorkflowActionInput, WorkflowActionResult};
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::Value;

/// Executes workflow `http_request` action nodes.
pub struct HttpRequestActionExecutor {
    client: Client,
    default_timeout: Duration,
}

impl HttpRequestActionExecutor {
    /// Creates a new HTTP action executor.
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            client: Client::new(),
            default_timeout,
        }
    }
}

impl Default for HttpRequestActionExecutor {
    fn default() -> Self {
        Self::new(Duration::from_secs(15))
    }
}

#[async_trait]
impl WorkflowActionExecutor for HttpRequestActionExecutor {
    fn action_type(&self) -> &'static str {
        "http_request"
    }

    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult> {
        let url = input
            .config
            .get("url")
            .and_then(Value::as_str)
            .context("http_request missing 'url' string")?;

        let method = input
            .config
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("POST")
            .parse::<Method>()
            .with_context(|| "invalid HTTP method")?;

        let timeout = input
            .config
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .map(Duration::from_millis)
            .unwrap_or(self.default_timeout);

        let mut request = self.client.request(method, url).timeout(timeout);

        if let Some(headers) = input.config.get("headers").and_then(Value::as_object) {
            for (name, value) in headers {
                if let Some(value_str) = value.as_str() {
                    request = request.header(name, value_str);
                }
            }
        }

        if let Some(body) = input.config.get("body") {
            request = request.json(body);
        } else if input
            .config
            .get("body_from_trigger")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            request = request.json(&input.trigger_payload);
        }

        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let body_snippet: String = body.chars().take(180).collect();

        if status.is_success() {
            Ok(WorkflowActionResult::success(format!(
                "http_request {} {url} ({})",
                status.as_u16(),
                if body_snippet.is_empty() {
                    "empty body"
                } else {
                    "body captured"
                }
            )))
        } else {
            Ok(WorkflowActionResult::failure(format!(
                "http_request failed {} {url}: {}",
                status.as_u16(),
                if body_snippet.is_empty() {
                    "no response body".to_string()
                } else {
                    body_snippet
                }
            )))
        }
    }
}
