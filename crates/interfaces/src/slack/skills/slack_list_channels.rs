//! `slack-list-channels` ambient tool.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackListChannelsSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackListChannelsSkill {
    fn name(&self) -> &str {
        "slack-list-channels"
    }

    fn description(&self) -> &str {
        "List Slack channels the bot can access. Optional: `limit` (max results, default 100)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "limit": { "type": "integer", "default": 100 }
            }
        })
    }

    async fn run(
        &self,
        params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as u32;

        match self.client.conversations_list(limit, None).await {
            Ok(resp) => {
                let channels = resp
                    .get("channels")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let truncated = resp
                    .pointer("/response_metadata/next_cursor")
                    .and_then(|v| v.as_str())
                    .map(|c| !c.is_empty())
                    .unwrap_or(false);
                let summary: Vec<serde_json::Value> = channels
                    .iter()
                    .map(|ch| {
                        json!({
                            "id": ch.get("id"),
                            "name": ch.get("name"),
                            "is_private": ch.get("is_private"),
                            "num_members": ch.get("num_members"),
                        })
                    })
                    .collect();
                let mut output = json!({ "channels": summary });
                if truncated {
                    output["truncated"] = json!(true);
                    output["note"] = json!("(results truncated, more channels exist)");
                }
                Ok(ToolOutput::success(serde_json::to_string_pretty(&output)?))
            }
            Err(e) => {
                warn!(error = %e, "slack-list-channels failed");
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

    fn skill_at(base: String) -> SlackListChannelsSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackListChannelsSkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_is_set() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-list-channels");
        assert!(s.params_schema().get("properties").is_some());
    }

    #[tokio::test]
    async fn run_returns_channel_summaries() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channels": [
                    {"id": "C1", "name": "general", "is_private": false, "num_members": 3},
                    {"id": "C2", "name": "random", "is_private": false, "num_members": 5}
                ],
                "response_metadata": {"next_cursor": ""}
            })))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("general"));
        assert!(out.content.contains("random"));
    }

    #[tokio::test]
    async fn run_marks_truncated_when_cursor_present() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "channels": [{"id": "C1", "name": "general"}],
                "response_metadata": {"next_cursor": "next-page"}
            })))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("truncated"));
    }

    #[tokio::test]
    async fn run_handles_empty_channels_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/conversations.list"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn run_reports_failure_when_transport_errors() {
        // Point at an unreachable address so the HTTP call fails.
        let s = skill_at("http://127.0.0.1:1".to_string());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);
    }
}
