//! `slack-react` ambient tool — add/remove emoji reactions.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use crate::client::SlackApiClient;

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
