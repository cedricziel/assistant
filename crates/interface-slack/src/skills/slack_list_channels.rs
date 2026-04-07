//! `slack-list-channels` ambient tool.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::warn;

use crate::client::SlackApiClient;

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
                Ok(ToolOutput::success(serde_json::to_string_pretty(&summary)?))
            }
            Err(e) => {
                warn!(error = %e, "slack-list-channels failed");
                Ok(ToolOutput::error(format!("Failed: {e}")))
            }
        }
    }
}
