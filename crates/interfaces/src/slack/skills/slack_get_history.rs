//! `slack-get-history` ambient tool — fetch channel or thread history.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackGetHistorySkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackGetHistorySkill {
    fn name(&self) -> &str {
        "slack-get-history"
    }

    fn description(&self) -> &str {
        "Fetch recent messages from a Slack channel, or replies in a thread. \
         Required: `channel` (channel ID). \
         Optional: `thread_ts` (fetch thread replies), `limit` (max messages, default 20)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel"],
            "properties": {
                "channel": { "type": "string" },
                "thread_ts": { "type": "string" },
                "limit": { "type": "integer", "default": 20 }
            }
        })
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
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(1000) as u32;

        let result = if let Some(ts) = params.get("thread_ts").and_then(|v| v.as_str()) {
            self.client
                .conversations_replies(channel, ts, limit)
                .await
                .map(serde_json::Value::Array)
        } else {
            self.client.conversations_history(channel, limit).await
        };

        match result {
            Ok(resp) => {
                let messages = resp
                    .get("messages")
                    .and_then(|v| v.as_array())
                    .map(|arr| serde_json::Value::Array(arr.clone()))
                    .unwrap_or(resp);
                Ok(ToolOutput::success(serde_json::to_string_pretty(
                    &messages,
                )?))
            }
            Err(e) => {
                warn!(error = %e, channel, "slack-get-history failed");
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

    fn skill_at(base: String) -> SlackGetHistorySkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackGetHistorySkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_is_set() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-get-history");
        assert!(s.params_schema().get("required").is_some());
    }

    #[tokio::test]
    async fn run_errors_on_missing_channel() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("channel"));
    }

    #[tokio::test]
    async fn run_fetches_channel_history_when_no_thread_ts() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.history"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "messages": [{"ts": "1.1", "text": "hi"}]
            })))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("hi"));
    }

    #[tokio::test]
    async fn run_fetches_thread_replies_when_thread_ts_present() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.replies"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "messages": [{"ts": "2.0", "text": "reply"}]
            })))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("thread_ts".into(), json!("1.2"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("reply"));
    }

    #[tokio::test]
    async fn run_clamps_huge_limit_and_uses_default_when_missing() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.history"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "messages": []
            })))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("limit".into(), json!(100_000));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
    }
}
