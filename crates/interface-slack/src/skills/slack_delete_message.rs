//! `slack-delete-message` ambient tool handler.
//!
//! Deletes a Slack message via `chat.delete`.  The bot can only delete its
//! own messages (unless it has the `chat:write:bot` admin scope).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackDeleteMessageSkill ───────────────────────────────────────────────────

pub struct SlackDeleteMessageSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackDeleteMessageSkill {
    fn name(&self) -> &str {
        "slack-delete-message"
    }

    fn description(&self) -> &str {
        "Delete a Slack message that the bot previously posted. \
         Required: `channel` (channel ID), `ts` (message timestamp). \
         The bot can only delete messages it originally posted."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts"],
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Slack channel ID (e.g. C01234567)"
                },
                "ts": {
                    "type": "string",
                    "description": "Timestamp of the message to delete"
                }
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
            Some(c) => c.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'channel'")),
        };
        let ts = match params.get("ts").and_then(|v| v.as_str()) {
            Some(t) => SlackTs(t.to_string()),
            None => return Ok(ToolOutput::error("Missing required parameter 'ts'")),
        };

        let session = self.client.open_session(&self.token);
        let req = SlackApiChatDeleteRequest::new(channel.clone().into(), ts.clone());

        match session.chat_delete(&req).await {
            Ok(resp) => {
                debug!(channel = %resp.channel, ts = %resp.ts.0, "slack-delete-message: ok");
                Ok(ToolOutput::success(format!(
                    "Message deleted (channel={}, ts={})",
                    resp.channel, resp.ts.0
                )))
            }
            Err(e) => {
                warn!(error = %e, channel = %channel, ts = %ts.0, "slack-delete-message: chat.delete failed");
                Ok(ToolOutput::error(format!("Failed to delete message: {e}")))
            }
        }
    }
}
