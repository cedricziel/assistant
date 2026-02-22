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
use tracing::{debug, warn};

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
        let content = SlackMessageContent::new().with_text(markdown_to_mrkdwn(&text));
        let mut req = SlackApiChatPostMessageRequest::new(self.channel_id.clone().into(), content);
        if let Some(ts) = &self.thread_ts {
            req = req.with_thread_ts(ts.clone());
        }

        match session.chat_post_message(&req).await {
            Ok(resp) => {
                debug!(channel = %resp.channel, ts = %resp.ts.0, "chat.postMessage ok");
                Ok(SkillOutput::success("Message posted successfully"))
            }
            Err(e) => {
                warn!(error = %e, "chat.postMessage failed");
                Ok(SkillOutput::error(format!("Failed to post message: {e}")))
            }
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

// ── Markdown → mrkdwn conversion ─────────────────────────────────────────────

/// Convert a subset of standard Markdown to Slack mrkdwn format.
///
/// Slack mrkdwn differs from Markdown in several key ways:
/// - Bold: `**text**` → `*text*`
/// - Italic: `*text*` → `_text_`
/// - Strikethrough: `~~text~~` → `~text~`
/// - Links: `[text](url)` → `<url|text>`
/// - Headings: `# heading` → `*heading*` (bold substitute)
///
/// Code spans and code blocks are passed through unchanged (Slack uses the
/// same backtick syntax). Unrecognised syntax is also passed through.
fn markdown_to_mrkdwn(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + 64);
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut i = 0;

    // Track whether we're at the start of a line for heading detection.
    let mut line_start = true;

    while i < n {
        // ATX headings: `# `, `## `, … at the start of a line.
        if line_start && chars[i] == '#' {
            let hash_start = i;
            while i < n && chars[i] == '#' {
                i += 1;
            }
            let hash_count = i - hash_start;
            if hash_count <= 6 && i < n && chars[i] == ' ' {
                i += 1; // skip the space after hashes
                        // Collect the rest of the line.
                let line_begin = i;
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                let heading: String = chars[line_begin..i].iter().collect();
                result.push('*');
                result.push_str(heading.trim_end());
                result.push('*');
                line_start = false;
                continue;
            } else {
                // Not a valid heading — emit the '#' chars as-is.
                for c in &chars[hash_start..i] {
                    result.push(*c);
                }
                line_start = false;
                continue;
            }
        }

        // Newline: reset line_start.
        if chars[i] == '\n' {
            result.push('\n');
            i += 1;
            line_start = true;
            continue;
        }
        line_start = false;

        // Fenced code block: ``` ... ``` — pass through unchanged.
        if chars[i] == '`' && chars.get(i + 1) == Some(&'`') && chars.get(i + 2) == Some(&'`') {
            let fence_end = find_closing_fence(&chars, i + 3);
            let end = fence_end.unwrap_or(n);
            for c in &chars[i..end] {
                result.push(*c);
            }
            i = end;
            continue;
        }

        // Inline code span: `...` — pass through unchanged.
        if chars[i] == '`' {
            let close = chars[i + 1..].iter().position(|&c| c == '`');
            let end = close.map(|p| i + 1 + p + 1).unwrap_or(n);
            for c in &chars[i..end] {
                result.push(*c);
            }
            i = end;
            continue;
        }

        // Markdown link: [text](url) → <url|text>
        if chars[i] == '[' {
            if let Some((text, url, advance)) = try_parse_link(&chars, i) {
                result.push('<');
                result.push_str(&url);
                result.push('|');
                result.push_str(&text);
                result.push('>');
                i += advance;
                continue;
            }
        }

        // Bold: **text** → *text*  (must check before single-star italic)
        if chars[i] == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some((content, advance)) = try_parse_span(&chars, i, 2) {
                result.push('*');
                result.push_str(&markdown_to_mrkdwn(&content));
                result.push('*');
                i += advance;
                continue;
            }
        }

        // Bold (underscore variant): __text__ → *text*  (check before single-underscore italic)
        if chars[i] == '_' && chars.get(i + 1) == Some(&'_') {
            if let Some((content, advance)) = try_parse_span(&chars, i, 2) {
                result.push('*');
                result.push_str(&markdown_to_mrkdwn(&content));
                result.push('*');
                i += advance;
                continue;
            }
        }

        // Italic: *text* → _text_  (single star, not preceded by another star)
        if chars[i] == '*' && chars.get(i + 1) != Some(&'*') {
            if let Some((content, advance)) = try_parse_span(&chars, i, 1) {
                result.push('_');
                result.push_str(&markdown_to_mrkdwn(&content));
                result.push('_');
                i += advance;
                continue;
            }
        }

        // Strikethrough: ~~text~~ → ~text~
        if chars[i] == '~' && chars.get(i + 1) == Some(&'~') {
            if let Some((content, advance)) = try_parse_span(&chars, i, 2) {
                result.push('~');
                result.push_str(&markdown_to_mrkdwn(&content));
                result.push('~');
                i += advance;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Locate the closing ` ``` ` fence starting at `from`, returning the index
/// just past the closing fence (including any trailing newline).
fn find_closing_fence(chars: &[char], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 2 < chars.len() {
        if chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`' {
            return Some(i + 3);
        }
        i += 1;
    }
    None
}

/// Try to parse a `[text](url)` Markdown link starting at `pos` (where
/// `chars[pos] == '['`).  Returns `(text, url, chars_consumed)` on success.
fn try_parse_link(chars: &[char], pos: usize) -> Option<(String, String, usize)> {
    // Find closing `]`
    let mut i = pos + 1;
    while i < chars.len() && chars[i] != ']' && chars[i] != '\n' {
        i += 1;
    }
    if i >= chars.len() || chars[i] != ']' {
        return None;
    }
    let text_end = i;
    // Expect `(` immediately after `]`
    if chars.get(i + 1) != Some(&'(') {
        return None;
    }
    i += 2;
    let url_start = i;
    while i < chars.len() && chars[i] != ')' && chars[i] != '\n' {
        i += 1;
    }
    if i >= chars.len() || chars[i] != ')' {
        return None;
    }
    let text: String = chars[pos + 1..text_end].iter().collect();
    let url: String = chars[url_start..i].iter().collect();
    if text.is_empty() || url.is_empty() {
        return None;
    }
    Some((text, url, i + 1 - pos))
}

/// Try to parse a delimited span starting at `pos` with a delimiter of
/// `delim_len` repeated characters (e.g. 2 for `**`, 1 for `*`).
/// Returns `(inner_content, chars_consumed)` on success.
///
/// The delimiter character is `chars[pos]`.  The span must not span newlines.
fn try_parse_span(chars: &[char], pos: usize, delim_len: usize) -> Option<(String, usize)> {
    let delim_char = chars[pos];
    // Verify opening delimiter.
    for k in 0..delim_len {
        if chars.get(pos + k) != Some(&delim_char) {
            return None;
        }
    }
    let inner_start = pos + delim_len;
    // Content must not start with the delimiter char or whitespace.
    if chars.get(inner_start) == Some(&delim_char) || chars.get(inner_start) == Some(&' ') {
        return None;
    }
    let mut i = inner_start;
    while i < chars.len() {
        if chars[i] == '\n' {
            return None; // spans must not cross line boundaries
        }
        // Check for closing delimiter (delim_len consecutive delim_chars not
        // preceded by whitespace).
        if chars[i] == delim_char && i > inner_start {
            let mut match_len = 0;
            while chars.get(i + match_len) == Some(&delim_char) {
                match_len += 1;
            }
            if match_len == delim_len && chars[i - 1] != ' ' {
                let content: String = chars[inner_start..i].iter().collect();
                return Some((content, i + delim_len - pos));
            }
        }
        i += 1;
    }
    None
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
                 Use this to send text responses to the user. \
                 Use Slack mrkdwn format (NOT standard Markdown): \
                 *bold*, _italic_, ~strikethrough~, `code`, \
                 ```code block``` for multi-line code, \
                 <url|link text> for hyperlinks. \
                 Do NOT use Markdown syntax (**bold**, *italic*, [text](url), # headings).",
                r#"{"type":"object","properties":{"text":{"type":"string","description":"Reply text in Slack mrkdwn format: *bold*, _italic_, ~strikethrough~, <url|text> for links — NOT standard Markdown"}},"required":["text"]}"#,
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

#[cfg(test)]
mod tests {
    use super::markdown_to_mrkdwn;

    #[test]
    fn bold_double_star_converted() {
        assert_eq!(markdown_to_mrkdwn("**hello**"), "*hello*");
    }

    #[test]
    fn italic_single_star_converted() {
        assert_eq!(markdown_to_mrkdwn("*hello*"), "_hello_");
    }

    #[test]
    fn bold_double_underscore_converted() {
        assert_eq!(markdown_to_mrkdwn("__hello__"), "*hello*");
    }

    #[test]
    fn italic_underscore_already_mrkdwn_unchanged() {
        // _italic_ is already valid Slack mrkdwn — must not be double-converted.
        assert_eq!(markdown_to_mrkdwn("_hello_"), "_hello_");
    }

    #[test]
    fn strikethrough_converted() {
        assert_eq!(markdown_to_mrkdwn("~~hello~~"), "~hello~");
    }

    #[test]
    fn link_converted() {
        assert_eq!(
            markdown_to_mrkdwn("[Rust](https://rust-lang.org)"),
            "<https://rust-lang.org|Rust>"
        );
    }

    #[test]
    fn heading_converted() {
        assert_eq!(markdown_to_mrkdwn("# My Heading"), "*My Heading*");
        assert_eq!(markdown_to_mrkdwn("## Sub Heading"), "*Sub Heading*");
    }

    #[test]
    fn inline_code_passed_through() {
        assert_eq!(markdown_to_mrkdwn("`code`"), "`code`");
    }

    #[test]
    fn bold_does_not_affect_italic_outcome() {
        // **bold** should become *bold*, and *italic* should become _italic_
        let input = "**bold** and *italic*";
        let output = markdown_to_mrkdwn(input);
        assert_eq!(output, "*bold* and _italic_");
    }

    #[test]
    fn plain_text_unchanged() {
        let input = "Just plain text here.";
        assert_eq!(markdown_to_mrkdwn(input), input);
    }

    #[test]
    fn mixed_content() {
        let input = "Check [this link](https://example.com) for **important** info.";
        let output = markdown_to_mrkdwn(input);
        assert_eq!(
            output,
            "Check <https://example.com|this link> for *important* info."
        );
    }

    #[test]
    fn mrkdwn_already_correct_unchanged() {
        // Text already in mrkdwn format should pass through without double-conversion.
        // Single *bold* in mrkdwn is converted to _bold_ (italic), which is expected
        // since we treat * as italic at the Markdown level. LLMs producing mrkdwn
        // directly should use the slack-reply description guidance.
        let input = "no special chars here";
        assert_eq!(markdown_to_mrkdwn(input), input);
    }

    #[test]
    fn slack_emoji_unchanged() {
        // Slack emoji syntax :name: must pass through the converter untouched.
        let input = "Hello! How can I assist you today? :wave:";
        assert_eq!(markdown_to_mrkdwn(input), input);
    }
}
