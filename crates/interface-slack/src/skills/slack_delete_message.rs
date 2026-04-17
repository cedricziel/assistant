//! `slack-delete-message` ambient tool — delete a Slack message.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::client::SlackApiClient;

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
