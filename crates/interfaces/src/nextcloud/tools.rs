//! Per-turn extension tools for the Nextcloud Talk interface.
//!
//! These tools are created fresh for each incoming message and are bound to
//! the specific conversation token and message ID of the triggering event.
//! They are registered as extension tools with the orchestrator so the LLM
//! can reply to or react to messages in Nextcloud Talk.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, warn};

use assistant_core::{ExecutionContext, ToolHandler, ToolOutput, sanitize_llm_output};

use super::signing::sign_request;

/// Default HTTP timeout for outgoing Nextcloud API requests.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Build a shared [`reqwest::Client`] with a default timeout.
fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

// ── Reply tool ───────────────────────────────────────────────────────────────

/// Per-turn tool that posts a reply message to the current Nextcloud Talk
/// conversation.
pub struct NextcloudReplyHandler {
    /// Nextcloud server base URL (e.g. `"https://nextcloud.example.com"`).
    server_url: String,
    /// Shared secret for signing requests.
    secret: String,
    /// Conversation token (room) to reply in.
    conversation_token: String,
    /// Message ID to reply to (the triggering message).
    reply_to_id: String,
    /// HTTP client.
    client: reqwest::Client,
}

impl NextcloudReplyHandler {
    pub fn new(
        server_url: String,
        secret: String,
        conversation_token: String,
        reply_to_id: String,
    ) -> Self {
        Self {
            server_url,
            secret,
            conversation_token,
            reply_to_id,
            client: build_client(),
        }
    }
}

#[async_trait]
impl ToolHandler for NextcloudReplyHandler {
    fn name(&self) -> &str {
        "reply"
    }

    fn description(&self) -> &str {
        "Send a reply message in the current Nextcloud Talk conversation. \
         Use this to respond to the user. The message will be posted as a \
         reply to the message that triggered this turn."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message text to send. Supports Markdown formatting."
                },
                "silent": {
                    "type": "boolean",
                    "description": "If true, the message will not create chat notifications.",
                    "default": false
                }
            },
            "required": ["message"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let raw_message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");

        let message = match sanitize_llm_output(raw_message) {
            Some(m) => m,
            None => {
                return Ok(ToolOutput::error(
                    "message is empty after sanitization (was it all <think> content?)",
                ));
            }
        };

        let silent = params
            .get("silent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let reply_to: i64 = self.reply_to_id.parse().unwrap_or(0);

        let mut body = serde_json::json!({
            "message": message,
        });
        if reply_to > 0 {
            body["replyTo"] = serde_json::json!(reply_to);
        }
        if silent {
            body["silent"] = serde_json::json!(true);
        }

        let body_str = body.to_string();
        let (random, signature) = match sign_request(&self.secret, &body_str) {
            Ok(pair) => pair,
            Err(e) => return Ok(ToolOutput::error(format!("Signing failed: {e}"))),
        };

        let url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/message",
            self.server_url.trim_end_matches('/'),
            self.conversation_token
        );

        debug!(url = %url, "Posting reply to Nextcloud Talk");

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("OCS-APIRequest", "true")
            .header("X-Nextcloud-Talk-Bot-Random", &random)
            .header("X-Nextcloud-Talk-Bot-Signature", &signature)
            .body(body_str)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(ToolOutput::success("Message sent")),
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                warn!(status = %status, body = %text, "Nextcloud reply failed");
                Ok(ToolOutput::error(format!(
                    "Failed to send reply: HTTP {status}: {text}"
                )))
            }
            Err(e) => {
                warn!(error = %e, "Nextcloud reply request failed");
                Ok(ToolOutput::error(format!("Request failed: {e}")))
            }
        }
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

// ── React tool ───────────────────────────────────────────────────────────────

/// Per-turn tool that adds an emoji reaction to the triggering message.
pub struct NextcloudReactHandler {
    /// Nextcloud server base URL.
    server_url: String,
    /// Shared secret for signing requests.
    secret: String,
    /// Conversation token.
    conversation_token: String,
    /// Message ID to react to.
    message_id: String,
    /// HTTP client.
    client: reqwest::Client,
}

impl NextcloudReactHandler {
    pub fn new(
        server_url: String,
        secret: String,
        conversation_token: String,
        message_id: String,
    ) -> Self {
        Self {
            server_url,
            secret,
            conversation_token,
            message_id,
            client: build_client(),
        }
    }
}

#[async_trait]
impl ToolHandler for NextcloudReactHandler {
    fn name(&self) -> &str {
        "nextcloud-react"
    }

    fn description(&self) -> &str {
        "Add an emoji reaction to the triggering message in Nextcloud Talk."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "reaction": {
                    "type": "string",
                    "description": "A single emoji to react with (e.g. \"\u{1F44D}\", \"\u{2764}\u{FE0F}\")."
                }
            },
            "required": ["reaction"]
        })
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let reaction = params
            .get("reaction")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if reaction.is_empty() {
            return Ok(ToolOutput::error("reaction parameter is required"));
        }

        let body = serde_json::json!({ "reaction": reaction });
        let body_str = body.to_string();
        let (random, signature) = match sign_request(&self.secret, &body_str) {
            Ok(pair) => pair,
            Err(e) => return Ok(ToolOutput::error(format!("Signing failed: {e}"))),
        };

        let url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v1/bot/{}/reaction/{}",
            self.server_url.trim_end_matches('/'),
            self.conversation_token,
            self.message_id
        );

        debug!(url = %url, reaction = %reaction, "Adding reaction in Nextcloud Talk");

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("OCS-APIRequest", "true")
            .header("X-Nextcloud-Talk-Bot-Random", &random)
            .header("X-Nextcloud-Talk-Bot-Signature", &signature)
            .body(body_str)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(ToolOutput::success("Reaction added")),
            Ok(r) => {
                let status = r.status();
                let text = r.text().await.unwrap_or_default();
                warn!(status = %status, body = %text, "Nextcloud react failed");
                Ok(ToolOutput::error(format!(
                    "Failed to add reaction: HTTP {status}: {text}"
                )))
            }
            Err(e) => {
                warn!(error = %e, "Nextcloud react request failed");
                Ok(ToolOutput::error(format!("Request failed: {e}")))
            }
        }
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

/// Build the per-turn extension tools for a specific Nextcloud Talk message.
pub fn build_nextcloud_tools(
    server_url: &str,
    secret: &str,
    conversation_token: &str,
    message_id: &str,
) -> Vec<Arc<dyn ToolHandler>> {
    vec![
        Arc::new(NextcloudReplyHandler::new(
            server_url.to_string(),
            secret.to_string(),
            conversation_token.to_string(),
            message_id.to_string(),
        )),
        Arc::new(NextcloudReactHandler::new(
            server_url.to_string(),
            secret.to_string(),
            conversation_token.to_string(),
            message_id.to_string(),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_handler_schema_has_required_fields() {
        let handler = NextcloudReplyHandler::new(
            "https://nc.local".into(),
            "secret".into(),
            "token".into(),
            "42".into(),
        );
        let schema = handler.params_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("message")));
        assert_eq!(handler.name(), "reply");
        assert!(handler.is_mutating());
    }

    #[test]
    fn react_handler_schema_has_required_fields() {
        let handler = NextcloudReactHandler::new(
            "https://nc.local".into(),
            "secret".into(),
            "token".into(),
            "42".into(),
        );
        let schema = handler.params_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("reaction")));
        assert_eq!(handler.name(), "nextcloud-react");
    }

    #[test]
    fn build_nextcloud_tools_returns_two_tools() {
        let tools = build_nextcloud_tools("https://nc.local", "secret", "token", "42");
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name(), "reply");
        assert_eq!(tools[1].name(), "nextcloud-react");
    }

    // ── Sanitization tests ───────────────────────────────────────────────

    #[test]
    fn sanitize_strips_think_tags() {
        let input = "<think>internal reasoning</think>Hello world";
        assert_eq!(sanitize_llm_output(input).unwrap(), "Hello world");
    }

    #[test]
    fn sanitize_strips_cite_tags_preserves_content() {
        let input = "See <cite index=\"1\">this source</cite> for details.";
        assert_eq!(
            sanitize_llm_output(input).unwrap(),
            "See this source for details."
        );
    }

    #[test]
    fn sanitize_only_think_returns_none() {
        let input = "<think>all hidden</think>";
        assert!(sanitize_llm_output(input).is_none());
    }

    #[test]
    fn sanitize_combined() {
        let input = "<think>hmm</think>Result: <cite index=\"1\">answer</cite>!";
        assert_eq!(sanitize_llm_output(input).unwrap(), "Result: answer!");
    }

    #[test]
    fn sanitize_plain_text_unchanged() {
        let input = "Hello, how can I help?";
        assert_eq!(sanitize_llm_output(input).unwrap(), input);
    }
}
