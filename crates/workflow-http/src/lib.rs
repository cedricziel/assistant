//! HTTP action executor for workflow nodes.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Context, Result};
use assistant_workflow::{WorkflowActionExecutor, WorkflowActionInput, WorkflowActionResult};
use async_trait::async_trait;
use reqwest::{Client, Method, Response};
use serde_json::Value;
use tokio::time::sleep;
use url::Url;

/// Executes workflow `http_request` action nodes.
pub struct HttpRequestActionExecutor {
    client: Client,
    default_timeout: Duration,
    default_retry_attempts: u32,
    default_retry_backoff: Duration,
    allowed_hosts: Option<HashSet<String>>,
    blocked_hosts: HashSet<String>,
}

impl HttpRequestActionExecutor {
    /// Creates a new HTTP action executor.
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            client: Client::new(),
            default_timeout,
            default_retry_attempts: 1,
            default_retry_backoff: Duration::from_millis(250),
            allowed_hosts: None,
            blocked_hosts: HashSet::new(),
        }
    }

    /// Restrict outbound requests to this host allowlist.
    pub fn with_allowed_hosts(mut self, hosts: impl IntoIterator<Item = String>) -> Self {
        self.allowed_hosts = Some(
            hosts
                .into_iter()
                .map(|host| host.to_ascii_lowercase())
                .collect(),
        );
        self
    }

    /// Block outbound requests to hosts in this denylist.
    pub fn with_blocked_hosts(mut self, hosts: impl IntoIterator<Item = String>) -> Self {
        self.blocked_hosts = hosts
            .into_iter()
            .map(|host| host.to_ascii_lowercase())
            .collect();
        self
    }

    /// Set default retry behavior.
    pub fn with_retry_defaults(mut self, attempts: u32, backoff: Duration) -> Self {
        self.default_retry_attempts = attempts.max(1);
        self.default_retry_backoff = backoff;
        self
    }

    fn enforce_host_policy(&self, url: &Url) -> Result<()> {
        let host = url
            .host_str()
            .map(|h| h.to_ascii_lowercase())
            .context("http_request url missing host")?;
        if self.blocked_hosts.contains(&host) {
            anyhow::bail!("host '{host}' is blocked by workflow policy");
        }
        if let Some(allowed_hosts) = &self.allowed_hosts
            && !allowed_hosts.contains(&host)
        {
            anyhow::bail!("host '{host}' is not in allowed workflow host list");
        }
        Ok(())
    }

    async fn map_response(
        &self,
        response_config: Option<&Value>,
        url: &str,
        response: Response,
    ) -> Result<WorkflowActionResult> {
        let status = response.status();
        let headers_obj: serde_json::Map<String, Value> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    Value::String(v.to_str().unwrap_or_default().to_string()),
                )
            })
            .collect();

        let body = response.text().await.unwrap_or_default();
        let body_snippet: String = body.chars().take(400).collect();
        let body_json = serde_json::from_str::<Value>(&body).ok();

        let mut output = serde_json::json!({
            "status": status.as_u16(),
            "ok": status.is_success(),
            "url": url,
            "headers": headers_obj,
            "body_text": body_snippet,
        });

        if let Some(parsed_json) = &body_json {
            output["body_json"] = parsed_json.clone();
            if let Some(pointer) = response_config
                .and_then(|cfg| cfg.get("json_pointer"))
                .and_then(Value::as_str)
            {
                output["mapped"] = parsed_json.pointer(pointer).cloned().unwrap_or(Value::Null);
            }
        }

        let note = format!("http_request {} {url}", status.as_u16());
        if status.is_success() {
            Ok(WorkflowActionResult::success(note).with_output(output))
        } else {
            Ok(WorkflowActionResult::failure(note).with_output(output))
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
        let parsed_url = Url::parse(url).context("http_request invalid url")?;
        self.enforce_host_policy(&parsed_url)?;

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

        let retry_attempts = input
            .config
            .get("retry")
            .and_then(|retry| retry.get("max_attempts"))
            .and_then(Value::as_u64)
            .and_then(|attempts| u32::try_from(attempts).ok())
            .unwrap_or(self.default_retry_attempts)
            .max(1);
        let retry_backoff = input
            .config
            .get("retry")
            .and_then(|retry| retry.get("backoff_ms"))
            .and_then(Value::as_u64)
            .map(Duration::from_millis)
            .unwrap_or(self.default_retry_backoff);

        let body = input.config.get("body").cloned();
        let body_from_trigger = input
            .config
            .get("body_from_trigger")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let headers = input
            .config
            .get("headers")
            .and_then(Value::as_object)
            .cloned();

        let mut last_error: Option<String> = None;
        for attempt in 1..=retry_attempts {
            let mut request = self.client.request(method.clone(), url).timeout(timeout);

            if let Some(headers) = &headers {
                for (name, value) in headers {
                    if let Some(value_str) = value.as_str() {
                        request = request.header(name, value_str);
                    }
                }
            }

            if let Some(body) = &body {
                request = request.json(body);
            } else if body_from_trigger {
                request = request.json(&input.trigger_payload);
            }

            match request.send().await {
                Ok(response) => {
                    return self
                        .map_response(input.config.get("response"), url, response)
                        .await;
                }
                Err(err) => {
                    last_error = Some(err.to_string());
                    if attempt < retry_attempts {
                        sleep(retry_backoff).await;
                    }
                }
            }
        }

        Ok(WorkflowActionResult::failure(format!(
            "http_request failed after {} attempt(s): {}",
            retry_attempts,
            last_error.unwrap_or_else(|| "unknown error".to_string())
        )))
    }
}
