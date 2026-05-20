//! `slack-update-message` ambient tool — edit an existing Slack message.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackUpdateMessageSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackUpdateMessageSkill {
    fn name(&self) -> &str {
        "slack-update-message"
    }

    fn description(&self) -> &str {
        "Update (edit) an existing Slack message in-place. \
         Required: `channel` (channel ID), `ts` (message timestamp), `text` (new text)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts", "text"],
            "properties": {
                "channel": { "type": "string" },
                "ts": { "type": "string" },
                "text": { "type": "string" }
            }
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let channel = match params.get("channel").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return Ok(ToolOutput::error("Missing 'channel'")),
        };
        let ts = match params.get("ts").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Ok(ToolOutput::error("Missing 'ts'")),
        };
        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Ok(ToolOutput::error("Missing 'text'")),
        };

        match self.client.update_message(channel, ts, text).await {
            Ok(()) => Ok(ToolOutput::success("Message updated")),
            Err(e) => {
                warn!(error = %e, "slack-update-message failed");
                Ok(ToolOutput::error(format!("Failed: {e}")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slack::skills::test_support::ctx;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn skill_at(base: String) -> SlackUpdateMessageSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackUpdateMessageSkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_marks_mutating() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-update-message");
        assert!(s.is_mutating());
        assert!(s.params_schema().get("required").is_some());
    }

    #[tokio::test]
    async fn run_updates_when_params_valid() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.update"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("ts".into(), json!("1.2"));
        params.insert("text".into(), json!("edited"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn run_errors_on_missing_params() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);

        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);

        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("ts".into(), json!("1.2"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("text"));
    }

    #[tokio::test]
    async fn run_propagates_client_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.update"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ok": false, "error": "bad"})),
            )
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("ts".into(), json!("1.2"));
        params.insert("text".into(), json!("x"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
    }
}
