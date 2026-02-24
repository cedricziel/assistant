//! Per-turn extension tools for the Mattermost interface.
//!
//! These tools are injected into the orchestrator via `run_turn_with_tools`
//! and capture Mattermost API context (channel, post_id, root_id, api client)
//! at construction time.  The LLM can call them to post replies or add
//! emoji reactions.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use mattermost_api::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::warn;

// ── MattermostReplyHandler ────────────────────────────────────────────────────

struct MattermostReplyHandler {
    channel_id: String,
    root_id: Option<String>,
    api: Arc<Mattermost>,
}

#[async_trait]
impl ToolHandler for MattermostReplyHandler {
    fn name(&self) -> &str {
        "mattermost-reply"
    }

    fn description(&self) -> &str {
        "Post a reply message in the current Mattermost channel or thread. \
         Use this to send text responses to the user."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Message text to post"
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
        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'text'")),
        };

        let body = mattermost_api::models::PostBody {
            channel_id: self.channel_id.clone(),
            message: text,
            root_id: self.root_id.clone(),
        };

        match self.api.create_post(&body).await {
            Ok(_) => Ok(ToolOutput::success("Message posted successfully")),
            Err(e) => Ok(ToolOutput::error(format!("Failed to post message: {e}"))),
        }
    }
}

// ── MattermostReactHandler ────────────────────────────────────────────────────

/// Minimal reaction body for the Mattermost `POST /reactions` endpoint.
#[derive(Debug, Serialize, Deserialize)]
struct ReactionBody {
    user_id: String,
    post_id: String,
    emoji_name: String,
}

/// Response type for `POST /reactions` — we only care that it succeeded.
#[derive(Debug, Serialize, Deserialize)]
struct ReactionResponse {}

struct MattermostReactHandler {
    post_id: String,
    bot_user_id: String,
    api: Arc<Mattermost>,
}

#[async_trait]
impl ToolHandler for MattermostReactHandler {
    fn name(&self) -> &str {
        "mattermost-react"
    }

    fn description(&self) -> &str {
        "Add an emoji reaction to the message that triggered this conversation."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["emoji"],
            "properties": {
                "emoji": {
                    "type": "string",
                    "description": "Emoji name without colons, e.g. thumbsup"
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
        let emoji = match params.get("emoji").and_then(|v| v.as_str()) {
            Some(e) => e.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'emoji'")),
        };

        let body = ReactionBody {
            user_id: self.bot_user_id.clone(),
            post_id: self.post_id.clone(),
            emoji_name: emoji,
        };

        match self
            .api
            .post::<ReactionBody, ReactionResponse>("reactions", None, &body)
            .await
        {
            Ok(_) => Ok(ToolOutput::success("Reaction added")),
            Err(e) => {
                let msg = e.to_string();
                // Mattermost returns an error if the reaction already exists.
                if msg.contains("exists") || msg.contains("already") || msg.contains("400") {
                    Ok(ToolOutput::success("Reaction already present"))
                } else {
                    warn!(error = %e, "Failed to add Mattermost reaction");
                    Ok(ToolOutput::error(format!("Failed to add reaction: {e}")))
                }
            }
        }
    }
}

// ── Public factory ────────────────────────────────────────────────────────────

/// Build the set of Mattermost-specific extension tools for one turn.
///
/// * `channel_id` — the channel to post in
/// * `post_id` — the `id` of the triggering post (used for reactions)
/// * `root_id` — the root post ID for threading (`None` for top-level replies)
/// * `bot_user_id` — the bot's own Mattermost user ID (required for reactions)
/// * `api` — shared Mattermost client
pub fn build_mattermost_tools(
    channel_id: String,
    post_id: String,
    root_id: Option<String>,
    bot_user_id: String,
    api: Arc<Mattermost>,
) -> Vec<Arc<dyn ToolHandler>> {
    vec![
        Arc::new(MattermostReplyHandler {
            channel_id,
            root_id,
            api: api.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(MattermostReactHandler {
            post_id,
            bot_user_id,
            api,
        }) as Arc<dyn ToolHandler>,
    ]
}
