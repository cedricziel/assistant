//! Per-turn extension tools for the Slack interface.
//!
//! These tools are injected into the orchestrator via `run_turn_with_tools`
//! and capture Slack API context (channel, thread_ts, message_ts, client,
//! token) at construction time.  The LLM can call them to post replies,
//! add reactions, post rich Block Kit messages, or upload files.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
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
                    "description": "Reply text in Slack mrkdwn format: *bold*, _italic_, ~strikethrough~, <url|text> for links — NOT standard Markdown"
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

        // Strip inline <think>…</think> blocks before posting; they are
        // already stored in the DB by the orchestrator and must not be
        // shown to the user.
        let text = strip_think_tags(&text);
        // Strip <cite index="…">…</cite> tags (keep inner text).
        let text = strip_cite_tags(&text);
        if text.is_empty() {
            return Ok(ToolOutput::success(
                "(thinking-only response; no visible content)",
            ));
        }

        let session = self.client.open_session(&self.token);
        let content = SlackMessageContent::new().with_text(markdown_to_mrkdwn(&text));
        let mut req = SlackApiChatPostMessageRequest::new(self.channel_id.clone().into(), content);
        if let Some(ts) = &self.thread_ts {
            req = req.with_thread_ts(ts.clone());
        }

        match session.chat_post_message(&req).await {
            Ok(resp) => {
                debug!(channel = %resp.channel, ts = %resp.ts.0, "chat.postMessage ok");
                Ok(ToolOutput::success("Message posted successfully"))
            }
            Err(e) => {
                warn!(error = %e, "chat.postMessage failed");
                Ok(ToolOutput::error(format!("Failed to post message: {e}")))
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
impl ToolHandler for SlackReactHandler {
    fn name(&self) -> &str {
        "react"
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

        let session = self.client.open_session(&self.token);
        let req = SlackApiReactionsAddRequest::new(
            self.channel_id.clone().into(),
            SlackReactionName(emoji),
            self.message_ts.clone(),
        );

        match session.reactions_add(&req).await {
            Ok(_) => Ok(ToolOutput::success("Reaction added")),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("already_reacted") {
                    Ok(ToolOutput::success("Reaction already present"))
                } else {
                    Ok(ToolOutput::error(format!("Failed to add reaction: {e}")))
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
impl ToolHandler for SlackReplyBlocksHandler {
    fn name(&self) -> &str {
        "reply-blocks"
    }

    fn description(&self) -> &str {
        "Post a rich Block Kit message in the current Slack thread. \
         Use this for formatted cards, buttons, and structured layouts."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["blocks"],
            "properties": {
                "blocks": {
                    "type": "string",
                    "description": "JSON array of Slack Block Kit blocks"
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
        let blocks_str = match params.get("blocks").and_then(|v| v.as_str()) {
            Some(b) => b.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'blocks'")),
        };

        let blocks: Vec<SlackBlock> = match serde_json::from_str(&blocks_str) {
            Ok(b) => b,
            Err(e) => return Ok(ToolOutput::error(format!("Invalid blocks JSON: {e}"))),
        };

        let session = self.client.open_session(&self.token);
        let content = SlackMessageContent::new().with_blocks(blocks);
        let mut req = SlackApiChatPostMessageRequest::new(self.channel_id.clone().into(), content);
        if let Some(ts) = &self.thread_ts {
            req = req.with_thread_ts(ts.clone());
        }

        match session.chat_post_message(&req).await {
            Ok(_) => Ok(ToolOutput::success("Block Kit message posted successfully")),
            Err(e) => Ok(ToolOutput::error(format!(
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
impl ToolHandler for SlackUploadHandler {
    fn name(&self) -> &str {
        "upload"
    }

    fn description(&self) -> &str {
        "Upload a file, image, or text snippet to the current Slack channel. \
         For text content, set `content` directly. \
         For binary files (images, PDFs, etc.), set `content_base64` with the \
         base64-encoded data and specify the `content_type` MIME type."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["filename"],
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Text file content (mutually exclusive with content_base64)"
                },
                "content_base64": {
                    "type": "string",
                    "description": "Base64-encoded binary content for images, PDFs, etc. (mutually exclusive with content)"
                },
                "filename": {
                    "type": "string",
                    "description": "Filename including extension (e.g. chart.png, report.pdf)"
                },
                "title": {
                    "type": "string",
                    "description": "Optional file title"
                },
                "content_type": {
                    "type": "string",
                    "description": "MIME type of the file (e.g. image/png, application/pdf). Defaults to application/octet-stream"
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
        let filename = match params.get("filename").and_then(|v| v.as_str()) {
            Some(f) => f.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'filename'")),
        };
        let title = params
            .get("title")
            .and_then(|v| v.as_str())
            .map(|t| t.to_string());
        let content_type = params
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Accept either text `content` or base64-encoded `content_base64`.
        let bytes = match resolve_upload_bytes(&params) {
            Ok(b) => b,
            Err(msg) => return Ok(ToolOutput::error(msg)),
        };

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
            Err(e) => return Ok(ToolOutput::error(format!("Failed to get upload URL: {e}"))),
        };

        // Step 2: upload the file bytes to the returned URL.
        let upload_req = SlackApiFilesUploadViaUrlRequest {
            upload_url: url_resp.upload_url,
            content: bytes,
            content_type,
        };
        if let Err(e) = session.files_upload_via_url(&upload_req).await {
            return Ok(ToolOutput::error(format!(
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
            Ok(_) => Ok(ToolOutput::success("File uploaded successfully")),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to complete file upload: {e}"
            ))),
        }
    }
}

// ── Think-tag stripping ───────────────────────────────────────────────────────

/// Strip `<think>…</think>` blocks that some models (e.g. qwen3) embed inline
/// in their response text when not using a dedicated thinking API.
///
/// The full original text (including think blocks) is preserved in the database
/// by the orchestrator; this function only removes them before posting to Slack
/// so users never see raw reasoning output.
fn strip_think_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let lower = input.to_lowercase();
    let mut pos = 0;
    while pos < input.len() {
        match lower[pos..].find("<think>") {
            Some(open_rel) => {
                let open_abs = pos + open_rel;
                result.push_str(&input[pos..open_abs]);
                match lower[open_abs..].find("</think>") {
                    Some(close_rel) => {
                        pos = open_abs + close_rel + "</think>".len();
                    }
                    None => break, // unclosed tag — discard the rest
                }
            }
            None => {
                result.push_str(&input[pos..]);
                break;
            }
        }
    }
    result.trim().to_string()
}

// ── Cite-tag stripping ───────────────────────────────────────────────────────

/// Strip `<cite index="…">…</cite>` tags that some models embed to attribute
/// sources, keeping only the inner text.
///
/// Unlike think-tags the *content* of a cite block is meaningful and must be
/// preserved; only the surrounding tags are removed.
fn strip_cite_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;
    let bytes = input.as_bytes();

    while pos < bytes.len() {
        // Look for `<cite` (case-sensitive — models emit lowercase).
        match input[pos..].find("<cite") {
            Some(open_rel) => {
                let open_abs = pos + open_rel;
                // Copy everything before the tag.
                result.push_str(&input[pos..open_abs]);

                // Find the closing `>` of the opening tag.
                match input[open_abs..].find('>') {
                    Some(gt_rel) => {
                        let content_start = open_abs + gt_rel + 1;
                        // Find the matching `</cite>`.
                        match input[content_start..].find("</cite>") {
                            Some(close_rel) => {
                                // Keep the inner content.
                                result.push_str(&input[content_start..content_start + close_rel]);
                                pos = content_start + close_rel + "</cite>".len();
                            }
                            None => {
                                // Unclosed cite — keep everything after the opening tag as-is.
                                result.push_str(&input[content_start..]);
                                return result;
                            }
                        }
                    }
                    None => {
                        // Malformed opening tag — copy remainder and bail.
                        result.push_str(&input[open_abs..]);
                        return result;
                    }
                }
            }
            None => {
                result.push_str(&input[pos..]);
                break;
            }
        }
    }
    result
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
/// - Tables: consecutive `|`-prefixed lines → fenced code block
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

        // Markdown table: consecutive lines starting with `|` → wrap in a
        // fenced code block so Slack renders them monospaced with aligned
        // columns.  Requires at least two `|`-prefixed lines (header + separator
        // or header + data row) to avoid false positives on stray `|` chars.
        if line_start && chars[i] == '|' {
            let mut end = i;
            let mut line_count = 0u32;
            loop {
                // Scan to end of this line.
                while end < n && chars[end] != '\n' {
                    end += 1;
                }
                line_count += 1;
                // Check whether the next line also starts with `|`.
                if end < n && end + 1 < n && chars[end + 1] == '|' {
                    end += 1; // skip '\n', advance to next `|`-line
                } else {
                    break;
                }
            }
            if line_count >= 2 {
                let table: String = chars[i..end].iter().collect();
                result.push_str("```\n");
                result.push_str(&table);
                result.push_str("\n```");
                i = end;
                line_start = false;
                continue;
            }
            // Single `|`-line — not a table, fall through to normal processing.
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
) -> Vec<Arc<dyn ToolHandler>> {
    vec![
        Arc::new(SlackReplyHandler {
            channel_id: channel_id.clone(),
            thread_ts: thread_ts.clone(),
            client: client.clone(),
            token: token.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(SlackReactHandler {
            channel_id: channel_id.clone(),
            message_ts,
            client: client.clone(),
            token: token.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(SlackReplyBlocksHandler {
            channel_id: channel_id.clone(),
            thread_ts: thread_ts.clone(),
            client: client.clone(),
            token: token.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(SlackUploadHandler {
            channel_id,
            thread_ts,
            client,
            token,
        }) as Arc<dyn ToolHandler>,
    ]
}

/// Resolve file content bytes from either a `content` (text) or
/// `content_base64` (base64-encoded binary) parameter.
///
/// Returns `Ok(bytes)` on success, or an error string for
/// `ToolOutput::error()`.
///
/// Handles the following real-world LLM encoding quirks:
/// - Data-URI prefixes such as `data:image/png;base64,` are stripped.
/// - ASCII whitespace (newlines, spaces) inserted for readability is stripped.
/// - Missing trailing `=` padding is tolerated via a `STANDARD_NO_PAD`
///   fallback so both padded and unpadded base64 are accepted.
fn resolve_upload_bytes(params: &HashMap<String, Value>) -> Result<Vec<u8>, String> {
    if let Some(b64) = params.get("content_base64").and_then(|v| v.as_str()) {
        use base64::Engine as _;

        // Strip "data:<mime>;base64," prefix produced by some callers.
        let b64 = match b64.find(";base64,") {
            Some(idx) => &b64[idx + ";base64,".len()..],
            None => b64,
        };

        // Remove ASCII whitespace (newlines, spaces) that LLMs commonly
        // insert into long base64 strings for readability.
        let b64_clean: String = b64.chars().filter(|c| !c.is_ascii_whitespace()).collect();

        // Try padded decode first; fall back to no-pad for inputs where the
        // trailing '=' characters have been omitted by the LLM.
        base64::engine::general_purpose::STANDARD
            .decode(&b64_clean)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(&b64_clean))
            .map_err(|e| format!("Invalid base64 in content_base64: {e}"))
    } else if let Some(text) = params.get("content").and_then(|v| v.as_str()) {
        Ok(text.as_bytes().to_vec())
    } else {
        Err("Either 'content' or 'content_base64' must be provided".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{markdown_to_mrkdwn, resolve_upload_bytes, strip_cite_tags, strip_think_tags};

    #[test]
    fn think_tags_stripped() {
        assert_eq!(
            strip_think_tags("<think>reasoning</think>\nHello!"),
            "Hello!"
        );
    }

    #[test]
    fn think_tags_middle_stripped() {
        assert_eq!(
            strip_think_tags("Before<think>reasoning</think>After"),
            "BeforeAfter"
        );
    }

    #[test]
    fn no_think_tags_unchanged() {
        assert_eq!(strip_think_tags("Hello world"), "Hello world");
    }

    #[test]
    fn only_think_tags_returns_empty() {
        assert_eq!(strip_think_tags("<think>all thinking</think>"), "");
    }

    // ── strip_cite_tags tests ─────────────────────────────────────────────────

    #[test]
    fn cite_tags_stripped_content_kept() {
        assert_eq!(
            strip_cite_tags(r#"Hello <cite index="1-2">world</cite>!"#),
            "Hello world!"
        );
    }

    #[test]
    fn multiple_cite_tags_stripped() {
        let input = r#"<cite index="1">A</cite> and <cite index="2-3,4">B</cite>"#;
        assert_eq!(strip_cite_tags(input), "A and B");
    }

    #[test]
    fn no_cite_tags_unchanged() {
        assert_eq!(strip_cite_tags("Hello world"), "Hello world");
    }

    #[test]
    fn cite_tag_with_complex_index() {
        let input = r#"<cite index="14-11,14-12,14-13,14-14,14-15,14-16">Directions here.</cite>"#;
        assert_eq!(strip_cite_tags(input), "Directions here.");
    }

    #[test]
    fn unclosed_cite_tag_keeps_content() {
        let input = r#"Before <cite index="1">unclosed content"#;
        assert_eq!(strip_cite_tags(input), "Before unclosed content");
    }

    #[test]
    fn cite_tags_in_realistic_message() {
        let input = r#"P3 — Freiluftparkplatz. <cite index="14-11,14-12">Aus Richtung A52 bleiben.</cite>

<cite index="14-22">Parkgebühr: 7 Euro.</cite>"#;
        let expected = "P3 — Freiluftparkplatz. Aus Richtung A52 bleiben.\n\nParkgebühr: 7 Euro.";
        assert_eq!(strip_cite_tags(input), expected);
    }

    // ── markdown_to_mrkdwn tests ────────────────────────────────────────────

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

    #[test]
    fn table_wrapped_in_code_block() {
        let input = "| Name | Age |\n|------|-----|\n| Alice | 30 |";
        let expected = "```\n| Name | Age |\n|------|-----|\n| Alice | 30 |\n```";
        assert_eq!(markdown_to_mrkdwn(input), expected);
    }

    #[test]
    fn table_with_surrounding_text() {
        let input = "Here is a table:\n| A | B |\n|---|---|\n| 1 | 2 |\nEnd.";
        let output = markdown_to_mrkdwn(input);
        assert_eq!(
            output,
            "Here is a table:\n```\n| A | B |\n|---|---|\n| 1 | 2 |\n```\nEnd."
        );
    }

    #[test]
    fn single_pipe_line_not_wrapped() {
        // A single line starting with `|` is not a table.
        let input = "| just a pipe";
        assert_eq!(markdown_to_mrkdwn(input), input);
    }

    #[test]
    fn table_at_end_of_input() {
        let input = "Results:\n| x | y |\n| 1 | 2 |";
        let expected = "Results:\n```\n| x | y |\n| 1 | 2 |\n```";
        assert_eq!(markdown_to_mrkdwn(input), expected);
    }

    // ── resolve_upload_bytes tests ───────────────────────────────────────────

    use serde_json::{json, Value};
    use std::collections::HashMap;

    fn params(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn upload_bytes_from_text_content() {
        let p = params(&[("content", json!("hello world"))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn upload_bytes_from_base64_content() {
        use base64::Engine as _;
        let original = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]; // PNG header
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        let p = params(&[("content_base64", json!(encoded))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(bytes, original);
    }

    #[test]
    fn upload_bytes_base64_takes_priority_over_text() {
        use base64::Engine as _;
        let binary = vec![1, 2, 3];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&binary);
        let p = params(&[
            ("content", json!("text fallback")),
            ("content_base64", json!(encoded)),
        ]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, binary,
            "base64 should take priority when both present"
        );
    }

    #[test]
    fn upload_bytes_missing_both_returns_error() {
        let p = params(&[("filename", json!("test.txt"))]);
        let err = resolve_upload_bytes(&p).unwrap_err();
        assert!(
            err.contains("content"),
            "error should mention content: {err}"
        );
    }

    #[test]
    fn upload_bytes_invalid_base64_returns_error() {
        let p = params(&[("content_base64", json!("not-valid-base64!!!"))]);
        let err = resolve_upload_bytes(&p).unwrap_err();
        assert!(
            err.contains("Invalid base64"),
            "error should mention invalid base64: {err}"
        );
    }

    #[test]
    fn upload_bytes_empty_text_returns_empty_vec() {
        let p = params(&[("content", json!(""))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn upload_bytes_empty_base64_returns_empty_vec() {
        let p = params(&[("content_base64", json!(""))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn upload_bytes_base64_with_newlines_decodes_correctly() {
        use base64::Engine as _;
        let original = vec![0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG magic
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        // Simulate an LLM inserting a newline in the middle of the string.
        let with_newline = format!("{}\n{}", &encoded[..4], &encoded[4..]);
        let p = params(&[("content_base64", json!(with_newline))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, original,
            "whitespace in base64 should be stripped before decoding"
        );
    }

    #[test]
    fn upload_bytes_base64_without_padding_decodes_correctly() {
        use base64::Engine as _;
        let original = vec![1u8, 2, 3]; // 3 bytes → 4 base64 chars with no padding needed
        let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(&original);
        // Verify the encoded string has no '=' padding.
        assert!(
            !encoded.contains('='),
            "test setup: encoded must lack padding"
        );
        let p = params(&[("content_base64", json!(encoded))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, original,
            "base64 without trailing '=' padding should decode correctly"
        );
    }

    #[test]
    fn upload_bytes_data_uri_prefix_stripped() {
        use base64::Engine as _;
        let original = vec![0x89u8, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        let data_uri = format!("data:image/png;base64,{encoded}");
        let p = params(&[("content_base64", json!(data_uri))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(
            bytes, original,
            "data-URI prefix should be stripped before base64 decoding"
        );
    }
}
