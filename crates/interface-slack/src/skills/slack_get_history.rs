//! `slack-get-history` ambient tool — fetch channel or thread history.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::warn;

use crate::client::SlackApiClient;

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
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;

        let result = if let Some(ts) = params.get("thread_ts").and_then(|v| v.as_str()) {
            self.client
                .conversations_replies(channel, ts)
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
