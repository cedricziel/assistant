//! `slack-post` builtin skill handler.
//!
//! Allows the agent to proactively post messages to any configured Slack channel
//! from any execution context (CLI turn, scheduled task, etc.).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::skill::{SkillDef, SkillOutput, SkillSource, SkillTier};
use assistant_core::{ExecutionContext, SkillHandler};
use async_trait::async_trait;
use slack_morphism::prelude::*;
use tracing::{debug, warn};

// ── SlackPostSkill ────────────────────────────────────────────────────────────

/// Skill handler that posts a message to an arbitrary Slack channel.
///
/// Unlike the per-turn `slack-reply` handler (which is pre-bound to the
/// current channel/thread), this handler accepts `channel` as a parameter
/// so the agent can post anywhere.
pub struct SlackPostSkill {
    pub(crate) client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    pub(crate) token: SlackApiToken,
}

#[async_trait]
impl SkillHandler for SlackPostSkill {
    fn skill_name(&self) -> &str {
        "slack-post"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let channel = match params.get("channel").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'channel'")),
        };
        let message = match params.get("message").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'message'")),
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
                Ok(SkillOutput::success(format!(
                    "Message posted to {} (ts={})",
                    resp.channel, resp.ts.0
                )))
            }
            Err(e) => {
                warn!(error = %e, channel = %channel, "slack-post: chat.postMessage failed");
                Ok(SkillOutput::error(format!("Failed to post message: {e}")))
            }
        }
    }
}

// ── SkillDef factory ──────────────────────────────────────────────────────────

/// Build the `SkillDef` for the `slack-post` ambient skill.
pub fn slack_post_def() -> SkillDef {
    let mut metadata = HashMap::new();
    metadata.insert("tier".to_string(), "builtin".to_string());

    SkillDef {
        name: "slack-post".to_string(),
        description: "Post a message to a Slack channel. Use this to proactively notify users, \
                       share results, or start conversations on Slack. \
                       Required parameters: `channel` (Slack channel ID or name, e.g. C01234567 \
                       or `#general`), `message` (text to post). \
                       Optional: `thread_ts` (timestamp of parent message to reply in-thread)."
            .to_string(),
        license: None,
        compatibility: None,
        allowed_tools: vec![],
        metadata,
        body: "Parameters:\n\
               - channel (string, required): Slack channel ID or name (e.g. C01234567 or #general)\n\
               - message (string, required): Message text to post (Slack mrkdwn formatting supported)\n\
               - thread_ts (string, optional): Timestamp of parent message to reply in-thread"
            .to_string(),
        dir: PathBuf::new(),
        tier: SkillTier::Builtin,
        mutating: true,
        confirmation_required: false,
        source: SkillSource::Builtin,
    }
}
