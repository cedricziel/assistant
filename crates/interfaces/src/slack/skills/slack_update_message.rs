//! `slack-update-message` ambient tool — edit an existing Slack message.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::slack::client::SlackApiClient;

pub struct SlackUpdateMessageSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackUpdateMessageSkill {
    fn name(&self) -> &str {
        "slack-update-message"
    }

    fn description(&self) -> &str {
        "Update (edit) an existing Slack message in-place. \
         Required: `channel` (channel ID), `ts` (message timestamp), `text` (new text)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts", "text"],
            "properties": {
                "channel": { "type": "string" },
                "ts": { "type": "string" },
                "text": { "type": "string" }
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
        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Ok(ToolOutput::error("Missing 'text'")),
        };

        match self.client.update_message(channel, ts, text).await {
            Ok(()) => Ok(ToolOutput::success("Message updated")),
            Err(e) => {
                warn!(error = %e, "slack-update-message failed");
                Ok(ToolOutput::error(format!("Failed: {e}")))
            }
        }
    }
}
