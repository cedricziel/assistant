//! Per-turn extension tools for the Slack interface.
//!
//! These tools are injected into the orchestrator via `run_turn_with_tools`
//! and capture Slack API context (channel, thread_ts, message_ts, client,
//! token) at construction time.  The LLM can call them to post replies,
//! add reactions, post rich Block Kit messages, or upload files.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::skill::SkillSource;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput, SkillTier};
use async_trait::async_trait;
use slack_morphism::prelude::*;

// ── SlackReplyHandler ─────────────────────────────────────────────────────────

struct SlackReplyHandler {
    channel_id: String,
    thread_ts: Option<SlackTs>,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    token: SlackApiToken,
}

#[async_trait]
impl SkillHandler for SlackReplyHandler {
    fn skill_name(&self) -> &str {
        "slack-reply"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let text = match params.get("text").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'text'")),
        };

        let session = self.client.open_session(&self.token);
        let content = SlackMessageContent::new().with_text(text);
        let mut req = SlackApiChatPostMessageRequest::new(self.channel_id.clone().into(), content);
        if let Some(ts) = &self.thread_ts {
            req = req.with_thread_ts(ts.clone());
        }

        match session.chat_post_message(&req).await {
            Ok(_) => Ok(SkillOutput::success("Message posted successfully")),
            Err(e) => Ok(SkillOutput::error(format!("Failed to post message: {e}"))),
        }
    }
}

// ── SlackReactHandler ─────────────────────────────────────────────────────────

struct SlackReactHandler {
    channel_id: String,
    message_ts: SlackTs,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    token: SlackApiToken,
}

#[async_trait]
impl SkillHandler for SlackReactHandler {
    fn skill_name(&self) -> &str {
        "slack-react"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let emoji = match params.get("emoji").and_then(|v| v.as_str()) {
            Some(e) => e.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'emoji'")),
        };

        let session = self.client.open_session(&self.token);
        let req = SlackApiReactionsAddRequest::new(
            self.channel_id.clone().into(),
            SlackReactionName(emoji),
            self.message_ts.clone(),
        );

        match session.reactions_add(&req).await {
            Ok(_) => Ok(SkillOutput::success("Reaction added")),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("already_reacted") {
                    Ok(SkillOutput::success("Reaction already present"))
                } else {
                    Ok(SkillOutput::error(format!("Failed to add reaction: {e}")))
                }
            }
        }
    }
}

// ── SlackReplyBlocksHandler ───────────────────────────────────────────────────

struct SlackReplyBlocksHandler {
    channel_id: String,
    thread_ts: Option<SlackTs>,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    token: SlackApiToken,
}

#[async_trait]
impl SkillHandler for SlackReplyBlocksHandler {
    fn skill_name(&self) -> &str {
        "slack-reply-blocks"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let blocks_str = match params.get("blocks").and_then(|v| v.as_str()) {
            Some(b) => b.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'blocks'")),
        };

        let blocks: Vec<SlackBlock> = match serde_json::from_str(&blocks_str) {
            Ok(b) => b,
            Err(e) => return Ok(SkillOutput::error(format!("Invalid blocks JSON: {e}"))),
        };

        let session = self.client.open_session(&self.token);
        let content = SlackMessageContent::new().with_blocks(blocks);
        let mut req = SlackApiChatPostMessageRequest::new(self.channel_id.clone().into(), content);
        if let Some(ts) = &self.thread_ts {
            req = req.with_thread_ts(ts.clone());
        }

        match session.chat_post_message(&req).await {
            Ok(_) => Ok(SkillOutput::success(
                "Block Kit message posted successfully",
            )),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to post Block Kit message: {e}"
            ))),
        }
    }
}

// ── SlackUploadHandler ────────────────────────────────────────────────────────

struct SlackUploadHandler {
    channel_id: String,
    thread_ts: Option<SlackTs>,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    token: SlackApiToken,
}

#[async_trait]
impl SkillHandler for SlackUploadHandler {
    fn skill_name(&self) -> &str {
        "slack-upload"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'content'")),
        };
        let filename = match params.get("filename").and_then(|v| v.as_str()) {
            Some(f) => f.to_string(),
            None => return Ok(SkillOutput::error("Missing required parameter 'filename'")),
        };
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .map(|t| t.to_string());

        let bytes = content.into_bytes();
        let length = bytes.len();

        let session = self.client.open_session(&self.token);

        // Step 1: request an upload URL from Slack.
        let url_req = SlackApiFilesGetUploadUrlExternalRequest {
            filename: filename.clone(),
            length,
            alt_txt: None,
            snippet_type: None,
        };
        let url_resp = match session.get_upload_url_external(&url_req).await {
            Ok(r) => r,
            Err(e) => return Ok(SkillOutput::error(format!("Failed to get upload URL: {e}"))),
        };

        // Step 2: upload the file bytes to the returned URL.
        let upload_req = SlackApiFilesUploadViaUrlRequest {
            upload_url: url_resp.upload_url,
            content: bytes,
            content_type: "application/octet-stream".to_string(),
        };
        if let Err(e) = session.files_upload_via_url(&upload_req).await {
            return Ok(SkillOutput::error(format!(
                "Failed to upload file content: {e}"
            )));
        }

        // Step 3: complete the upload and share to the channel.
        let complete_req = SlackApiFilesCompleteUploadExternalRequest {
            files: vec![SlackApiFilesComplete {
                id: url_resp.file_id,
                title,
            }],
            channel_id: Some(self.channel_id.clone().into()),
            initial_comment: None,
            thread_ts: self.thread_ts.clone(),
        };

        match session.files_complete_upload_external(&complete_req).await {
            Ok(_) => Ok(SkillOutput::success("File uploaded successfully")),
            Err(e) => Ok(SkillOutput::error(format!(
                "Failed to complete file upload: {e}"
            ))),
        }
    }
}

// ── Factory helpers ───────────────────────────────────────────────────────────

fn make_skill_def(name: &str, description: &str, params_json: &str) -> SkillDef {
    let mut metadata = HashMap::new();
    metadata.insert("tier".to_string(), "builtin".to_string());
    metadata.insert("params".to_string(), params_json.to_string());
    SkillDef {
        name: name.to_string(),
        description: description.to_string(),
        license: None,
        compatibility: None,
        allowed_tools: vec![],
        metadata,
        body: String::new(),
        dir: PathBuf::new(),
        tier: SkillTier::Builtin,
        mutating: false,
        confirmation_required: false,
        source: SkillSource::Builtin,
    }
}

// ── Public factory ────────────────────────────────────────────────────────────

/// Build the set of Slack-specific extension tools for one turn.
///
/// * `channel_id` — the channel to post/react in
/// * `thread_ts` — the thread to reply into (pass `Some(thread_ts)` to thread,
///   `None` for top-level posts)
/// * `message_ts` — the `ts` of the triggering message (used for reactions)
/// * `client` — shared Slack HTTP client
/// * `token` — bot token used for API authentication
pub fn build_slack_tools(
    channel_id: String,
    thread_ts: Option<SlackTs>,
    message_ts: SlackTs,
    client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
    token: SlackApiToken,
) -> Vec<(SkillDef, Arc<dyn SkillHandler>)> {
    vec![
        (
            make_skill_def(
                "slack-reply",
                "Post a reply message in the current Slack thread. \
                 Use this to send text responses to the user.",
                r#"{"type":"object","properties":{"text":{"type":"string","description":"Message text to post"}},"required":["text"]}"#,
            ),
            Arc::new(SlackReplyHandler {
                channel_id: channel_id.clone(),
                thread_ts: thread_ts.clone(),
                client: client.clone(),
                token: token.clone(),
            }) as Arc<dyn SkillHandler>,
        ),
        (
            make_skill_def(
                "slack-react",
                "Add an emoji reaction to the message that triggered this conversation.",
                r#"{"type":"object","properties":{"emoji":{"type":"string","description":"Emoji name without colons, e.g. thumbsup"}},"required":["emoji"]}"#,
            ),
            Arc::new(SlackReactHandler {
                channel_id: channel_id.clone(),
                message_ts,
                client: client.clone(),
                token: token.clone(),
            }) as Arc<dyn SkillHandler>,
        ),
        (
            make_skill_def(
                "slack-reply-blocks",
                "Post a rich Block Kit message in the current Slack thread. \
                 Use this for formatted cards, buttons, and structured layouts.",
                r#"{"type":"object","properties":{"blocks":{"type":"string","description":"JSON array of Slack Block Kit blocks"}},"required":["blocks"]}"#,
            ),
            Arc::new(SlackReplyBlocksHandler {
                channel_id: channel_id.clone(),
                thread_ts: thread_ts.clone(),
                client: client.clone(),
                token: token.clone(),
            }) as Arc<dyn SkillHandler>,
        ),
        (
            make_skill_def(
                "slack-upload",
                "Upload a file or text snippet to the current Slack channel.",
                r#"{"type":"object","properties":{"content":{"type":"string","description":"File content"},"filename":{"type":"string","description":"Filename including extension"},"title":{"type":"string","description":"Optional file title"}},"required":["content","filename"]}"#,
            ),
            Arc::new(SlackUploadHandler {
                channel_id,
                thread_ts,
                client,
                token,
            }) as Arc<dyn SkillHandler>,
        ),
    ]
}
