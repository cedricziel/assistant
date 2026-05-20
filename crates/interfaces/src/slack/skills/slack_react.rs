//! `slack-react` ambient tool — add/remove emoji reactions.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackReactSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackReactSkill {
    fn name(&self) -> &str {
        "slack-react"
    }

    fn description(&self) -> &str {
        "Add or remove an emoji reaction on a Slack message. \
         Required: `channel`, `ts` (message timestamp), `emoji`. \
         Optional: `action` (`add` or `remove`, default: `add`)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts", "emoji"],
            "properties": {
                "channel": { "type": "string" },
                "ts": { "type": "string" },
                "emoji": { "type": "string" },
                "action": { "type": "string", "enum": ["add", "remove"] }
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
        let emoji = match params.get("emoji").and_then(|v| v.as_str()) {
            Some(e) => e.trim_matches(':'),
            None => return Ok(ToolOutput::error("Missing 'emoji'")),
        };

        match self.client.add_reaction(channel, ts, emoji).await {
            Ok(()) => Ok(ToolOutput::success(format!(":{emoji}: reaction added"))),
            Err(e) => {
                warn!(error = %e, "slack-react failed");
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

    fn skill_at(base: String) -> SlackReactSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackReactSkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_is_set() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-react");
        assert!(s.is_mutating());
        assert!(s.params_schema().get("required").is_some());
    }

    #[tokio::test]
    async fn run_strips_colons_from_emoji_and_adds_reaction() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok":true})))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("ts".into(), json!("1.2"));
        params.insert("emoji".into(), json!(":thumbsup:"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("thumbsup"));
    }

    #[tokio::test]
    async fn run_errors_on_missing_params() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("channel"));

        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("ts"));

        let mut params = HashMap::new();
        params.insert("channel".into(), json!("C1"));
        params.insert("ts".into(), json!("1.2"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("emoji"));
    }

    #[tokio::test]
    async fn run_propagates_client_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
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
        params.insert("emoji".into(), json!("ok"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
    }
}
