//! `slack-lookup-user` ambient tool — look up Slack user info.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
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
