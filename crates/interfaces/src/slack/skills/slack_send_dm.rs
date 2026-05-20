//! `slack-send-dm` ambient tool — open a DM and send a message.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackSendDmSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackSendDmSkill {
    fn name(&self) -> &str {
        "slack-send-dm"
    }

    fn description(&self) -> &str {
        "Open a DM channel with a Slack user and send them a message. \
         Required: `user_id` (Slack user ID, e.g. U01234567), `message`."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["user_id", "message"],
            "properties": {
                "user_id": { "type": "string" },
                "message": { "type": "string" }
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
        let user_id = match params.get("user_id").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => return Ok(ToolOutput::error("Missing 'user_id'")),
        };
        let message = match params.get("message").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => return Ok(ToolOutput::error("Missing 'message'")),
        };

        // Open a DM channel via conversations.open
        let body = json!({ "users": user_id });

        #[derive(Deserialize)]
        struct OpenResp {
            ok: bool,
            channel: Option<serde_json::Value>,
            error: Option<String>,
        }

        let resp: OpenResp = match self
            .client
            .http_post_json("/api/conversations.open", &body)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "slack-send-dm: conversations.open failed");
                return Ok(ToolOutput::error(format!("Failed to open DM: {e}")));
            }
        };

        if !resp.ok {
            return Ok(ToolOutput::error(format!(
                "conversations.open error: {}",
                resp.error.unwrap_or_default()
            )));
        }

        let channel_id = resp
            .channel
            .and_then(|c| c.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .unwrap_or_default();

        match self.client.post_message(&channel_id, message, None).await {
            Ok(ts) => Ok(ToolOutput::success(format!(
                "DM sent to {user_id} (ts={ts})"
            ))),
            Err(e) => {
                warn!(error = %e, user_id, "slack-send-dm: post_message failed");
                Ok(ToolOutput::error(format!("Failed to send DM: {e}")))
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

    fn skill_at(base: String) -> SlackSendDmSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackSendDmSkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_is_set() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-send-dm");
        assert!(s.is_mutating());
    }

    #[tokio::test]
    async fn run_errors_on_missing_params() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);

        let mut params = HashMap::new();
        params.insert("user_id".into(), json!("U1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("message"));
    }

    #[tokio::test]
    async fn run_opens_dm_then_posts_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/conversations.open"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channel": {"id": "D1"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true,"ts":"5.5"})),
            )
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("user_id".into(), json!("U1"));
        params.insert("message".into(), json!("hi"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("DM sent"));
    }

    #[tokio::test]
    async fn run_returns_error_when_conversations_open_fails() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/conversations.open"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ok": false, "error": "user_not_found"})),
            )
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("user_id".into(), json!("UX"));
        params.insert("message".into(), json!("hi"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("conversations.open"));
    }

    #[tokio::test]
    async fn run_returns_error_when_post_message_fails() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/conversations.open"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channel": {"id": "D1"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/chat.postMessage"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ok": false, "error": "bad"})),
            )
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("user_id".into(), json!("U1"));
        params.insert("message".into(), json!("hi"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("send DM"));
    }
}
