//! `slack-react` ambient tool handler.
//!
//! Adds or removes an emoji reaction on any Slack message.  Unlike the
//! per-turn `slack-react` extension tool (which is pre-bound to the current
//! message), this ambient tool accepts arbitrary `channel` / `ts` parameters
//! so the agent can react to any message in any channel.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackReactSkill ───────────────────────────────────────────────────────────

pub struct SlackReactSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackReactSkill {
    fn name(&self) -> &str {
        "slack-react"
    }

    fn description(&self) -> &str {
        "Add or remove an emoji reaction on a Slack message. \
         Required: `channel` (channel ID), `ts` (message timestamp), \
         `emoji` (emoji name without colons, e.g. `thumbsup`, `white_check_mark`). \
         Optional: `action` (`add` or `remove`, default: `add`)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["channel", "ts", "emoji"],
            "properties": {
                "channel": {
                    "type": "string",
                    "description": "Slack channel ID (e.g. C01234567)"
                },
                "ts": {
                    "type": "string",
                    "description": "Timestamp of the message to react to"
                },
                "emoji": {
                    "type": "string",
                    "description": "Emoji name without colons (e.g. thumbsup, white_check_mark)"
                },
                "action": {
                    "type": "string",
                    "enum": ["add", "remove"],
                    "description": "Whether to add or remove the reaction (default: add)"
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
        let emoji = match params.get("emoji").and_then(|v| v.as_str()) {
            Some(e) => e.trim_matches(':').to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'emoji'")),
        };
        let remove = params
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| s == "remove")
            .unwrap_or(false);

        let session = self.client.open_session(&self.token);
        let reaction = SlackReactionName(emoji.clone());

        if remove {
            let req = SlackApiReactionsRemoveRequest::new(reaction)
                .with_channel(channel.clone().into())
                .with_timestamp(ts);
            match session.reactions_remove(&req).await {
                Ok(_) => {
                    debug!(channel = %channel, emoji = %emoji, "slack-react: reaction removed");
                    Ok(ToolOutput::success(format!(
                        "Removed :{emoji}: reaction from message"
                    )))
                }
                Err(e) => {
                    warn!(error = %e, "slack-react: reactions.remove failed");
                    Ok(ToolOutput::error(format!("Failed to remove reaction: {e}")))
                }
            }
        } else {
            let req = SlackApiReactionsAddRequest::new(channel.clone().into(), reaction, ts);
            match session.reactions_add(&req).await {
                Ok(_) => {
                    debug!(channel = %channel, emoji = %emoji, "slack-react: reaction added");
                    Ok(ToolOutput::success(format!(
                        "Added :{emoji}: reaction to message"
                    )))
                }
                Err(e) => {
                    // Slack returns already_reacted if the bot already added this reaction;
                    // treat it as a success since the desired state is already achieved.
                    let msg = e.to_string();
                    if msg.contains("already_reacted") {
                        debug!(emoji = %emoji, "slack-react: already_reacted, ignoring");
                        Ok(ToolOutput::success(format!(
                            ":{emoji}: reaction already present"
                        )))
                    } else {
                        warn!(error = %e, "slack-react: reactions.add failed");
                        Ok(ToolOutput::error(format!("Failed to add reaction: {e}")))
                    }
                }
            }
        }
    }
}
