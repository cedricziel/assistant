//! `slack-update-message` ambient tool handler.
//!
//! Edits the text of a previously posted Slack message via `chat.update`.
//! The bot can only update messages it originally posted.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackUpdateMessageSkill ───────────────────────────────────────────────────

pub struct SlackUpdateMessageSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackUpdateMessageSkill {
    fn name(&self) -> &str {
        "slack-update-message"
    }

    fn description(&self) -> &str {
        "Edit the text of a Slack message that the bot previously posted. \
         Required: `channel` (channel ID), `ts` (message timestamp returned \
         when the message was originally posted), `message` (new text content). \
         The bot can only update its own messages."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts", "message"],
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Slack channel ID (e.g. C01234567)"
                },
                "ts": {
                    "type": "string",
                    "description": "Timestamp of the message to update"
                },
                "message": {
                    "type": "string",
                    "description": "New message text (Slack mrkdwn formatting supported)"
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
        let message = match params.get("message").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'message'")),
        };

        let session = self.client.open_session(&self.token);
        let content = SlackMessageContent::new().with_text(message);
        let req = SlackApiChatUpdateRequest::new(channel.clone().into(), content, ts.clone());

        match session.chat_update(&req).await {
            Ok(resp) => {
                debug!(channel = %resp.channel, ts = %resp.ts.0, "slack-update-message: ok");
                Ok(ToolOutput::success(format!(
                    "Message updated (channel={}, ts={})",
                    resp.channel, resp.ts.0
                )))
            }
            Err(e) => {
                warn!(error = %e, channel = %channel, ts = %ts.0, "slack-update-message: chat.update failed");
                Ok(ToolOutput::error(format!("Failed to update message: {e}")))
            }
        }
    }
}
