//! `slack-get-history` ambient tool handler.
//!
//! Reads recent messages from a Slack channel (`conversations.history`) or, when
//! `thread_ts` is supplied, the replies in a specific thread
//! (`conversations.replies`).  Useful for the agent to catch up on context
//! before composing a response.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackGetHistorySkill ──────────────────────────────────────────────────────

pub struct SlackGetHistorySkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackGetHistorySkill {
    fn name(&self) -> &str {
        "slack-get-history"
    }

    fn description(&self) -> &str {
        "Read recent messages from a Slack channel or thread. \
         Required: `channel` (channel ID, e.g. C01234567). \
         Optional: `limit` (number of messages to return, default 20, max 100), \
         `thread_ts` (parent message timestamp — when set, returns thread replies \
         instead of channel history)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel"],
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Slack channel ID (e.g. C01234567)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of messages to return (default: 20, max: 100)"
                },
                "thread_ts": {
                    "type": "string",
                    "description": "Parent message timestamp — returns thread replies when set"
                }
            }
        })
    }

    fn is_mutating(&self) -> bool {
        false
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
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n.min(100))
            .unwrap_or(20) as u16;
        let thread_ts = params
            .get("thread_ts")
            .and_then(|v| v.as_str())
            .map(|s| SlackTs(s.to_string()));

        let session = self.client.open_session(&self.token);

        let messages: Vec<SlackHistoryMessage> = if let Some(ts) = thread_ts {
            // Fetch thread replies.
            let req = SlackApiConversationsRepliesRequest::new(channel.clone().into(), ts)
                .with_limit(limit);
            match session.conversations_replies(&req).await {
                Ok(resp) => resp.messages,
                Err(e) => {
                    warn!(error = %e, channel = %channel, "slack-get-history: conversations.replies failed");
                    return Ok(ToolOutput::error(format!("Failed to fetch thread: {e}")));
                }
            }
        } else {
            // Fetch channel history (most recent first from the API; we reverse to
            // present oldest-first to the LLM for readability).
            let req = SlackApiConversationsHistoryRequest::new()
                .with_channel(channel.clone().into())
                .with_limit(limit);
            match session.conversations_history(&req).await {
                Ok(resp) => {
                    let mut msgs = resp.messages;
                    msgs.reverse();
                    msgs
                }
                Err(e) => {
                    warn!(error = %e, channel = %channel, "slack-get-history: conversations.history failed");
                    return Ok(ToolOutput::error(format!("Failed to fetch history: {e}")));
                }
            }
        };

        debug!(count = messages.len(), channel = %channel, "slack-get-history: fetched messages");

        if messages.is_empty() {
            return Ok(ToolOutput::success("No messages found.".to_string()));
        }

        let lines: Vec<String> = messages
            .iter()
            .filter_map(|msg| {
                let text = msg.content.text.as_deref().filter(|t| !t.is_empty())?;
                let ts = &msg.origin.ts.0;
                let sender = if let Some(bot_id) = &msg.sender.bot_id {
                    format!("[bot {}]", bot_id.0)
                } else if let Some(user_id) = &msg.sender.user {
                    format!("@{}", user_id.0)
                } else {
                    "[unknown]".to_string()
                };
                Some(format!("[{ts}] {sender}: {text}"))
            })
            .collect();

        if lines.is_empty() {
            Ok(ToolOutput::success("No text messages found.".to_string()))
        } else {
            Ok(ToolOutput::success(lines.join("\n")))
        }
    }
}
