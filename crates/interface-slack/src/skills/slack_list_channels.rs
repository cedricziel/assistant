//! `slack-list-channels` ambient tool handler.
//!
//! Lists the Slack channels the bot is a member of using `conversations.list`
//! with cursor-based pagination.  For IM (direct-message) channels the creator
//! user's display name is resolved via `users.info` to make the output
//! human-readable.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackListChannelsSkill ────────────────────────────────────────────────────

/// Tool handler that lists Slack channels the bot is a member of.
pub struct SlackListChannelsSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackListChannelsSkill {
    fn name(&self) -> &str {
        "slack-list-channels"
    }

    fn description(&self) -> &str {
        "List Slack channels the bot is a member of. Returns channel IDs, names, and types. \
         Use this to discover valid channel targets for slack-post. \
         Optional: `types` (comma-separated: public_channel, private_channel, im, mpim; \
         default: public_channel,private_channel), `limit` (max results; default: 200)."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "types": {
                    "type": "string",
                    "description": "Comma-separated channel types to include. \
                                    Accepted values: public_channel, private_channel, im, mpim. \
                                    Default: public_channel,private_channel"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of channels to return (default: 200)"
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
        let types_str = params
            .get("types")
            .and_then(|v| v.as_str())
            .unwrap_or("public_channel,private_channel")
            .to_string();

        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize;

        // Parse requested channel types; silently skip unknown values.
        let types: Vec<SlackConversationType> = types_str
            .split(',')
            .map(|s| s.trim())
            .filter_map(parse_conversation_type)
            .collect();

        let include_im = types.iter().any(|t| matches!(t, SlackConversationType::Im));

        let session = self.client.open_session(&self.token);

        // ── Paginate conversations.list ───────────────────────────────────────
        let mut channels: Vec<SlackChannelInfo> = Vec::new();
        let mut cursor: Option<SlackCursorId> = None;

        loop {
            if channels.len() >= limit {
                break;
            }

            let page_limit = (limit - channels.len()).min(200) as u16;
            let mut req = SlackApiConversationsListRequest::new()
                .with_limit(page_limit)
                .with_exclude_archived(false)
                .with_types(types.clone());
            if let Some(c) = cursor.take() {
                req = req.with_cursor(c);
            }

            match session.conversations_list(&req).await {
                Ok(resp) => {
                    channels.extend(resp.channels);
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
                    warn!(error = %e, "slack-list-channels: conversations.list failed");
                    return Ok(ToolOutput::error(format!("Failed to list channels: {e}")));
                }
            }
        }

        channels.truncate(limit);
        debug!(
            count = channels.len(),
            "slack-list-channels: fetched channels"
        );

        if channels.is_empty() {
            return Ok(ToolOutput::success("No channels found.".to_string()));
        }

        // ── Format output ─────────────────────────────────────────────────────
        let mut lines: Vec<String> = Vec::with_capacity(channels.len());

        for ch in &channels {
            let type_label = channel_type_label(&ch.flags);
            let ch_id = ch.id.to_string();

            if include_im && ch.flags.is_im.unwrap_or(false) {
                // For DM channels the `name` field is empty; resolve the user.
                let user_label = if let Some(uid) = &ch.creator {
                    let info_req = SlackApiUsersInfoRequest::new(uid.clone());
                    match session.users_info(&info_req).await {
                        Ok(resp) => {
                            let display = resp
                                .user
                                .profile
                                .as_ref()
                                .and_then(|p| p.display_name.as_deref())
                                .filter(|s| !s.is_empty())
                                .or_else(|| {
                                    resp.user.real_name.as_deref().filter(|s| !s.is_empty())
                                })
                                .unwrap_or(&uid.0)
                                .to_string();
                            format!("@{display}")
                        }
                        Err(_) => format!("user:{}", uid.0),
                    }
                } else {
                    "(unknown user)".to_string()
                };
                lines.push(format!("[{type_label}] {ch_id} {user_label}"));
            } else {
                let name = ch
                    .name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .map(|n| format!("#{n}"))
                    .unwrap_or_else(|| ch_id.clone());
                lines.push(format!("[{type_label}] {ch_id} {name}"));
            }
        }

        Ok(ToolOutput::success(lines.join("\n")))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_conversation_type(s: &str) -> Option<SlackConversationType> {
    match s {
        "im" => Some(SlackConversationType::Im),
        "mpim" => Some(SlackConversationType::Mpim),
        "private_channel" => Some(SlackConversationType::Private),
        "public_channel" => Some(SlackConversationType::Public),
        _ => None,
    }
}

fn channel_type_label(flags: &SlackChannelFlags) -> &'static str {
    if flags.is_im.unwrap_or(false) {
        "im"
    } else if flags.is_mpim.unwrap_or(false) {
        "mpim"
    } else if flags.is_private.unwrap_or(false) {
        "private"
    } else {
        "public"
    }
}
