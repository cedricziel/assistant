//! `slack-send-dm` ambient tool handler.
//!
//! Allows the agent to send a direct message to any Slack user by ID or name.
//! Resolves display/real names via `users.list` when a plain name is given,
//! then opens (or reuses) a DM channel via `conversations.open` before
//! calling `chat.postMessage`.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackSendDmSkill ──────────────────────────────────────────────────────────

/// Tool handler that sends a direct message to a Slack user.
///
/// If `user` looks like a Slack ID (`U…` / `W…`), it is used directly.
/// Otherwise `users.list` is searched by display name, real name, or handle.
pub struct SlackSendDmSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackSendDmSkill {
    fn name(&self) -> &str {
        "slack-send-dm"
    }

    fn description(&self) -> &str {
        "Send a direct message to a Slack user. \
         Required parameters: `user` (Slack user ID like U01234567, or display/real name), \
         `message` (text to send, Slack mrkdwn supported). \
         Optional: `thread_ts` (reply in an existing DM thread)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["user", "message"],
            "properties": {
                "user": {
                    "type": "string",
                    "description": "Slack user ID (e.g. U01234567) or display/real name"
                },
                "message": {
                    "type": "string",
                    "description": "Message text (Slack mrkdwn formatting supported)"
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
        let user = match params.get("user").and_then(|v| v.as_str()) {
            Some(u) => u.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'user'")),
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

        // Resolve user ID if not already in Slack ID format (U… / W…).
        let user_id = if looks_like_user_id(&user) {
            user.clone()
        } else {
            match resolve_user_by_name(&session, &user).await {
                Some(id) => id,
                None => {
                    return Ok(ToolOutput::error(format!(
                        "Could not find Slack user: {user}"
                    )))
                }
            }
        };

        // Open (or retrieve) the DM channel for this user.
        let open_req =
            SlackApiConversationsOpenRequest::new().with_users(vec![SlackUserId(user_id.clone())]);
        let dm_channel_id = match session.conversations_open(&open_req).await {
            Ok(resp) => resp.channel.id.to_string(),
            Err(e) => {
                warn!(error = %e, user = %user_id, "slack-send-dm: conversations.open failed");
                return Ok(ToolOutput::error(format!("Failed to open DM channel: {e}")));
            }
        };

        // Post the message to the DM channel.
        let content = SlackMessageContent::new().with_text(message);
        let mut req = SlackApiChatPostMessageRequest::new(dm_channel_id.clone().into(), content);
        if let Some(ts) = thread_ts {
            req = req.with_thread_ts(ts);
        }

        match session.chat_post_message(&req).await {
            Ok(resp) => {
                debug!(channel = %resp.channel, ts = %resp.ts.0, "slack-send-dm: DM sent ok");
                Ok(ToolOutput::success(format!(
                    "DM sent to user {} via channel {} (ts={})",
                    user_id, resp.channel, resp.ts.0
                )))
            }
            Err(e) => {
                warn!(error = %e, user = %user_id, "slack-send-dm: chat.postMessage failed");
                Ok(ToolOutput::error(format!("Failed to send DM: {e}")))
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns `true` if `s` looks like a raw Slack user ID (`U…` or `W…`,
/// 8+ alphanumeric characters, optionally prefixed with `@`).
fn looks_like_user_id(s: &str) -> bool {
    let s = s.trim_start_matches('@');
    let upper = s.to_uppercase();
    (upper.starts_with('U') || upper.starts_with('W'))
        && s.len() >= 8
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Searches `users.list` for a member whose display name, real name, or
/// handle matches `name` (case-insensitive, leading `@` stripped).
/// Returns the Slack user ID (`U…`) on success.
async fn resolve_user_by_name(
    session: &SlackClientSession<'_, SlackClientHyperHttpsConnector>,
    name: &str,
) -> Option<String> {
    let target = name.trim_start_matches('@').to_lowercase();
    let mut cursor: Option<SlackCursorId> = None;

    loop {
        let mut req = SlackApiUsersListRequest::new().with_limit(200);
        if let Some(c) = cursor.take() {
            req = req.with_cursor(c);
        }

        match session.users_list(&req).await {
            Ok(resp) => {
                for member in &resp.members {
                    let display = member
                        .profile
                        .as_ref()
                        .and_then(|p| p.display_name.as_deref())
                        .unwrap_or("")
                        .to_lowercase();
                    let real = member
                        .profile
                        .as_ref()
                        .and_then(|p| p.real_name.as_deref())
                        .or(member.real_name.as_deref())
                        .unwrap_or("")
                        .to_lowercase();
                    let handle = member.name.as_deref().unwrap_or("").to_lowercase();

                    if display == target || real == target || handle == target {
                        return Some(member.id.0.clone());
                    }
                }

                let next = resp
                    .response_metadata
                    .as_ref()
                    .and_then(|m| m.next_cursor.as_ref())
                    .filter(|c| !c.0.is_empty())
                    .cloned();

                match next {
                    Some(c) => cursor = Some(c),
                    None => break,
                }
            }
            Err(e) => {
                warn!(error = %e, "slack-send-dm: users.list failed during name resolution");
                break;
            }
        }
    }

    None
}
