//! `slack-lookup-user` ambient tool — look up Slack user info.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackLookupUserSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackLookupUserSkill {
    fn name(&self) -> &str {
        "slack-lookup-user"
    }

    fn description(&self) -> &str {
        "Look up a Slack user's profile by their user ID. Required: `user_id`."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["user_id"],
            "properties": {
                "user_id": { "type": "string" }
            }
        })
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

        match self.client.users_info(user_id).await {
            Ok(resp) => {
                let user = resp.get("user").cloned().unwrap_or(resp);
                let summary = json!({
                    "id": user.get("id"),
                    "name": user.get("name"),
                    "real_name": user.pointer("/profile/real_name"),
                    "display_name": user.pointer("/profile/display_name"),
                    "is_bot": user.get("is_bot"),
                });
                Ok(ToolOutput::success(serde_json::to_string_pretty(&summary)?))
            }
            Err(e) => {
                warn!(error = %e, user_id, "slack-lookup-user failed");
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

    fn skill_at(base: String) -> SlackLookupUserSkill {
        let client = SlackApiClient::with_base_url("xoxb-t".into(), "xapp-t".into(), base).unwrap();
        SlackLookupUserSkill {
            client: Arc::new(client),
        }
    }

    #[test]
    fn metadata_is_set() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        assert_eq!(s.name(), "slack-lookup-user");
        assert!(s.params_schema().get("required").is_some());
    }

    #[tokio::test]
    async fn run_errors_on_missing_user_id() {
        let s = skill_at("http://127.0.0.1:0".to_string());
        let out = s.run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("user_id"));
    }

    #[tokio::test]
    async fn run_summarises_user_profile() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users.info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "user": {
                    "id": "U1",
                    "name": "alice",
                    "is_bot": false,
                    "profile": {"real_name": "Alice", "display_name": "ali"}
                }
            })))
            .mount(&server)
            .await;
        let s = skill_at(server.uri());
        let mut params = HashMap::new();
        params.insert("user_id".into(), json!("U1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("alice"));
        assert!(out.content.contains("Alice"));
    }

    #[tokio::test]
    async fn run_reports_failure_when_transport_errors() {
        let s = skill_at("http://127.0.0.1:1".to_string());
        let mut params = HashMap::new();
        params.insert("user_id".into(), json!("U1"));
        let out = s.run(params, &ctx()).await.unwrap();
        assert!(!out.success);
    }
}
