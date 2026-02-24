//! `slack-post` builtin tool handler.
//!
//! Allows the agent to proactively post messages to any configured Slack channel
//! from any execution context (CLI turn, scheduled task, etc.).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackPostSkill ────────────────────────────────────────────────────────────

/// Tool handler that posts a message to an arbitrary Slack channel.
///
/// Unlike the per-turn `slack-reply` handler (which is pre-bound to the
/// current channel/thread), this handler accepts `channel` as a parameter
/// so the agent can post anywhere.
pub struct SlackPostSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackPostSkill {
    fn name(&self) -> &str {
        "slack-post"
    }

    fn description(&self) -> &str {
        "Post a message to a Slack channel. Use this to proactively notify users, \
         share results, or start conversations on Slack. \
         Required parameters: `channel` (Slack channel ID or name, e.g. C01234567 \
         or `#general`), `message` (text to post). \
         Optional: `thread_ts` (timestamp of parent message to reply in-thread)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "message"],
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Slack channel ID or name (e.g. C01234567 or #general)"
                },
                "message": {
                    "type": "string",
                    "description": "Message text to post (Slack mrkdwn formatting supported)"
                },
                "thread_ts": {
                    "type": "string",
                    "description": "Timestamp of parent message to reply in-thread"
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
        let message = match params.get("message").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'message'")),
        };
        let thread_ts = params
            .get("thread_ts")
            .and_then(|v| v.as_str())
            .map(|s| SlackTs(s.to_string()));

        let session = self.client.open_session(&self.token);
        let content = SlackMessageContent::new().with_text(message);
        let mut req = SlackApiChatPostMessageRequest::new(channel.clone().into(), content);
        if let Some(ts) = thread_ts {
            req = req.with_thread_ts(ts);
        }

        match session.chat_post_message(&req).await {
            Ok(resp) => {
                debug!(channel = %resp.channel, ts = %resp.ts.0, "slack-post: chat.postMessage ok");
                Ok(ToolOutput::success(format!(
                    "Message posted to {} (ts={})",
                    resp.channel, resp.ts.0
                )))
            }
            Err(e) => {
                warn!(error = %e, channel = %channel, "slack-post: chat.postMessage failed");
                Ok(ToolOutput::error(format!("Failed to post message: {e}")))
            }
        }
    }
}
