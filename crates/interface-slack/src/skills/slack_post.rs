//! `slack-post` ambient tool — post to any Slack channel.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::client::SlackApiClient;

pub struct SlackPostSkill {
    pub(crate) client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackPostSkill {
    fn name(&self) -> &str {
        "slack-post"
    }

    fn description(&self) -> &str {
        "Post a message to a Slack channel. Required: `channel` (Slack channel ID, \
         e.g. C01234567), `message`. Optional: `thread_ts` (reply in-thread)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "message"],
            "properties": {
                "channel": { "type": "string" },
                "message": { "type": "string" },
                "thread_ts": { "type": "string" }
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
        let message = match params.get("message").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => return Ok(ToolOutput::error("Missing 'message'")),
        };
        let thread_ts = params.get("thread_ts").and_then(|v| v.as_str());

        match self.client.post_message(channel, message, thread_ts).await {
            Ok(ts) => Ok(ToolOutput::success(format!("Posted (ts={ts})"))),
            Err(e) => {
                warn!(error = %e, channel, "slack-post failed");
                Ok(ToolOutput::error(format!("Failed: {e}")))
            }
        }
    }
}
