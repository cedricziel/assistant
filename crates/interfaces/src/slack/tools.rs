//! Per-turn extension tools for the Slack interface.
//!
//! These tools are injected into the orchestrator via `run_turn_with_tools`
//! and capture Slack API context (channel, thread_ts, API client) at
//! construction time.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ToolHandler, ToolOutput, strip_cite_tags, strip_think_tags,
    types::conversation::ExecutionContext,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use super::client::SlackApiClient;

// ── SlackReplyHandler ─────────────────────────────────────────────────────────

pub struct SlackReplyHandler {
    pub channel_id: String,
    pub thread_ts: Option<String>,
    pub client: Arc<SlackApiClient>,
    /// If set, update this placeholder message in-place via `chat.update`.
    pub update_ts: Option<String>,
}

#[async_trait]
impl ToolHandler for SlackReplyHandler {
    fn name(&self) -> &str {
        "reply"
    }

    fn description(&self) -> &str {
        "Post a reply message in the current Slack thread. \
         Use this to send text responses to the user. \
         Use Slack mrkdwn format (NOT standard Markdown): \
         *bold*, _italic_, ~strikethrough~, `code`, \
         ```code block``` for multi-line code, \
         <url|link text> for hyperlinks. \
         Do NOT use Markdown syntax (**bold**, *italic*, [text](url), # headings)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The message text to post (Slack mrkdwn format)"
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
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let text = strip_think_tags(&strip_cite_tags(text));

        if let Some(ts) = &self.update_ts {
            // Update placeholder in-place.
            if let Err(e) = self
                .client
                .update_message(&self.channel_id, ts, &text)
                .await
            {
                warn!(error = %e, "chat.update failed; falling back to new message");
                self.client
                    .post_message(&self.channel_id, &text, self.thread_ts.as_deref())
                    .await?;
            }
        } else {
            self.client
                .post_message(&self.channel_id, &text, self.thread_ts.as_deref())
                .await?;
        }
        Ok(ToolOutput::success("Message posted"))
    }
}

// ── SlackReactHandler ─────────────────────────────────────────────────────────

pub struct SlackReactHandler {
    pub channel_id: String,
    pub message_ts: String,
    pub client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackReactHandler {
    fn name(&self) -> &str {
        "react"
    }

    fn description(&self) -> &str {
        "Add an emoji reaction to the current message."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["emoji"],
            "properties": {
                "emoji": {
                    "type": "string",
                    "description": "Emoji name without colons (e.g. 'thumbsup', 'white_check_mark')"
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
        let emoji = params
            .get("emoji")
            .and_then(|v| v.as_str())
            .unwrap_or("thumbsup");
        self.client
            .add_reaction(&self.channel_id, &self.message_ts, emoji)
            .await?;
        Ok(ToolOutput::success("Reaction added"))
    }
}

// ── SlackUploadHandler ────────────────────────────────────────────────────────

pub struct SlackUploadHandler {
    pub channel_id: String,
    pub thread_ts: Option<String>,
    pub client: Arc<SlackApiClient>,
}

#[async_trait]
impl ToolHandler for SlackUploadHandler {
    fn name(&self) -> &str {
        "upload"
    }

    fn description(&self) -> &str {
        "Upload a file to the current Slack channel/thread."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["filename", "content"],
            "properties": {
                "filename": { "type": "string" },
                "content": {
                    "type": "string",
                    "description": "File content as a UTF-8 string"
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
        let filename = params
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or("file.txt");
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        self.client
            .upload_file(
                &self.channel_id,
                filename,
                content.as_bytes().to_vec(),
                self.thread_ts.as_deref(),
            )
            .await?;
        Ok(ToolOutput::success("File uploaded"))
    }
}

// ── build_slack_tools ─────────────────────────────────────────────────────────

/// Build the set of per-turn extension tools for a Slack message.
pub fn build_slack_tools(
    channel_id: String,
    thread_ts: Option<String>,
    message_ts: String,
    client: Arc<SlackApiClient>,
) -> Vec<Box<dyn ToolHandler>> {
    vec![
        Box::new(SlackReplyHandler {
            channel_id: channel_id.clone(),
            thread_ts: thread_ts.clone(),
            client: client.clone(),
            update_ts: None,
        }),
        Box::new(SlackReactHandler {
            channel_id: channel_id.clone(),
            message_ts,
            client: client.clone(),
        }),
        Box::new(SlackUploadHandler {
            channel_id,
            thread_ts,
            client,
        }),
    ]
}
