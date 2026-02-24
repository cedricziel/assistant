//! `slack-lookup-user` ambient tool handler.
//!
//! Returns profile information (display name, real name, email, timezone,
//! title) for a Slack user, looked up by ID or by display/real name.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackLookupUserSkill ──────────────────────────────────────────────────────

pub struct SlackLookupUserSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl ToolHandler for SlackLookupUserSkill {
    fn name(&self) -> &str {
        "slack-lookup-user"
    }

    fn description(&self) -> &str {
        "Look up a Slack user's profile information. \
         Returns display name, real name, email, timezone, and title. \
         Required: `user` (Slack user ID like U01234567, or display/real name). \
         Leading `@` is stripped automatically."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["user"],
            "properties": {
                "user": {
                    "type": "string",
                    "description": "Slack user ID (e.g. U01234567) or display/real name"
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
        let user = match params.get("user").and_then(|v| v.as_str()) {
            Some(u) => u.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'user'")),
        };

        let session = self.client.open_session(&self.token);

        // Resolve to a user ID if a name was provided.
        let user_id = if looks_like_user_id(&user) {
            user.trim_start_matches('@').to_string()
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

        let req = SlackApiUsersInfoRequest::new(SlackUserId(user_id.clone()));
        match session.users_info(&req).await {
            Ok(resp) => {
                debug!(user = %user_id, "slack-lookup-user: users.info ok");
                Ok(ToolOutput::success(format_user(&resp.user)))
            }
            Err(e) => {
                warn!(error = %e, user = %user_id, "slack-lookup-user: users.info failed");
                Ok(ToolOutput::error(format!("Failed to look up user: {e}")))
            }
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn looks_like_user_id(s: &str) -> bool {
    let s = s.trim_start_matches('@');
    let upper = s.to_uppercase();
    (upper.starts_with('U') || upper.starts_with('W'))
        && s.len() >= 8
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

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
                warn!(error = %e, "slack-lookup-user: users.list failed during name resolution");
                break;
            }
        }
    }

    None
}

fn format_user(user: &SlackUser) -> String {
    let mut parts: Vec<String> = Vec::new();

    parts.push(format!("ID: {}", user.id.0));

    if let Some(name) = &user.name {
        parts.push(format!("Handle: @{name}"));
    }

    let profile = user.profile.as_ref();

    if let Some(display) = profile
        .and_then(|p| p.display_name.as_deref())
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("Display name: {display}"));
    }

    let real_name = profile
        .and_then(|p| p.real_name.as_deref())
        .filter(|s| !s.is_empty())
        .or_else(|| user.real_name.as_deref().filter(|s| !s.is_empty()));
    if let Some(rn) = real_name {
        parts.push(format!("Real name: {rn}"));
    }

    if let Some(title) = profile
        .and_then(|p| p.title.as_deref())
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("Title: {title}"));
    }

    if let Some(email) = profile.and_then(|p| p.email.as_ref()) {
        parts.push(format!("Email: {email}"));
    }

    if let Some(tz_label) = &user.tz_label {
        parts.push(format!("Timezone: {tz_label}"));
    }

    if user.flags.is_bot.unwrap_or(false) {
        parts.push("Type: bot".to_string());
    }

    if user.deleted.unwrap_or(false) {
        parts.push("Status: deactivated".to_string());
    }

    parts.join("\n")
}
