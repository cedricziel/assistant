//! `slack-post` ambient tool — post to any Slack channel.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackPostSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackPostSkill {
    fn name(&self) -> &str {
        "slack-post"
    }

    fn description(&self) -> &str {
        "Post a message to a Slack channel. Required: `channel` (Slack channel ID, \
         e.g. C01234567), `message`. Optional: `thread_ts` (reply in-thread)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "message"],
            "properties": {
                "channel": { "type": "string" },
                "message": { "type": "string" },
                "thread_ts": { "type": "string" }
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
        let message = match params.get("message").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => return Ok(ToolOutput::error("Missing 'message'")),
        };
        let thread_ts = params.get("thread_ts").and_then(|v| v.as_str());

        match self.client.post_message(channel, message, thread_ts).await {
            Ok(ts) => Ok(ToolOutput::success(format!("Posted (ts={ts})"))),
            Err(e) => {
                warn!(error = %e, channel, "slack-post failed");
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

    fn skill_with_base(base: String) -> SlackPostSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackPostSkill {
            client: Arc::new(client),
        }
    }

    fn skill(server: &MockServer) -> SlackPostSkill {
        skill_with_base(server.uri())
    }

    #[test]
    fn metadata_and_schema_are_set() {
        let s = skill_with_base("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-post");
        assert!(s.is_mutating());
        assert!(s.description().contains("Slack channel"));
        assert!(s.params_schema().get("required").is_some());
    }

    #[tokio::test]
    async fn run_posts_message_when_params_valid() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true,"ts":"1.2"})),
            )
            .mount(&server)
            .await;
        let s = skill(&server);
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("message".into(), json!("hello"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("Posted"));
    }

    #[tokio::test]
    async fn run_errors_when_channel_missing() {
        let server = MockServer::start().await;
        let s = skill(&server);
        let mut params = HashMap::new();
        params.insert("message".into(), json!("hello"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("channel"));
    }

    #[tokio::test]
    async fn run_errors_when_message_missing() {
        let server = MockServer::start().await;
        let s = skill(&server);
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("message"));
    }

    #[tokio::test]
    async fn run_reports_failure_when_client_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ok": false, "error": "channel_not_found"})),
            )
            .mount(&server)
            .await;
        let s = skill(&server);
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("CX"));
        params.insert("message".into(), json!("hello"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
    }
}
