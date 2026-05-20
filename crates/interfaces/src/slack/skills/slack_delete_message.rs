//! `slack-delete-message` ambient tool — delete a Slack message.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackDeleteMessageSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackDeleteMessageSkill {
    fn name(&self) -> &str {
        "slack-delete-message"
    }

    fn description(&self) -> &str {
        "Delete a Slack message. Required: `channel` (channel ID), `ts` (message timestamp)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts"],
            "properties": {
                "channel": { "type": "string" },
                "ts": { "type": "string" }
            }
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn requires_confirmation(&self) -> bool {
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

        match self.client.delete_message(channel, ts).await {
            Ok(()) => Ok(ToolOutput::success("Message deleted")),
            Err(e) => {
                warn!(error = %e, "slack-delete-message failed");
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

    fn skill_at(base: String) -> SlackDeleteMessageSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackDeleteMessageSkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_marks_mutating_and_requires_confirmation() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-delete-message");
        assert!(s.is_mutating());
        assert!(s.requires_confirmation());
    }

    #[tokio::test]
    async fn run_deletes_when_params_valid() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.delete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("ts".into(), json!("1.2"));
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
        assert!(out.content.contains("ts"));
    }
}
