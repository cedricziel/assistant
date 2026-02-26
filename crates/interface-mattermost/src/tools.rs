//! Per-turn extension tools for the Mattermost interface.
//!
//! These tools are injected into the orchestrator via `run_turn_with_tools`
//! and capture Mattermost API context (channel, post_id, root_id, api client)
//! at construction time.  The LLM can call them to post replies, add
//! emoji reactions, or upload files.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use mattermost_api::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, warn};

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

// ── MattermostUploadHandler ───────────────────────────────────────────────────

/// Minimal response for `POST /api/v4/files` — we need the file_infos.
#[derive(Debug, Deserialize)]
struct FileUploadResponse {
    file_infos: Vec<FileInfo>,
}

#[derive(Debug, Deserialize)]
struct FileInfo {
    id: String,
}

struct MattermostUploadHandler {
    channel_id: String,
    root_id: Option<String>,
    /// Base URL of the Mattermost server (e.g. `"https://mattermost.example.com"`).
    server_url: String,
    /// Auth token for API calls.
    auth_token: String,
    /// Shared Mattermost client for creating posts with file_ids.
    api: Arc<Mattermost>,
}

/// Post body with optional file_ids for attaching uploaded files.
#[derive(Debug, Serialize)]
struct PostBodyWithFiles {
    channel_id: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    file_ids: Vec<String>,
}

/// Minimal post response — we only care about success.
#[derive(Debug, Deserialize)]
struct PostResponse {}

#[async_trait]
impl ToolHandler for MattermostUploadHandler {
    fn name(&self) -> &str {
        "mattermost-upload"
    }

    fn description(&self) -> &str {
        "Upload a file, image, or document to the current Mattermost channel. \
         For text content, set `content` directly. \
         For binary files (images, PDFs, etc.), set `content_base64` with the \
         base64-encoded data."
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
                "message": {
                    "type": "string",
                    "description": "Optional message text to accompany the file"
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
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Accept either text `content` or base64-encoded `content_base64`.
        let bytes = match resolve_upload_bytes(&params) {
            Ok(b) => b,
            Err(msg) => return Ok(ToolOutput::error(msg)),
        };

        // Step 1: Upload file via multipart POST to /api/v4/files
        let url = format!("{}/api/v4/files", self.server_url.trim_end_matches('/'));

        let http = reqwest::Client::new();
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename.clone())
            .mime_str("application/octet-stream")
            .unwrap_or_else(|_| {
                reqwest::multipart::Part::bytes(Vec::new()).file_name(filename.clone())
            });

        let form = reqwest::multipart::Form::new()
            .text("channel_id", self.channel_id.clone())
            .part("files", part);

        let resp = http
            .post(&url)
            .bearer_auth(&self.auth_token)
            .multipart(form)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => return Ok(ToolOutput::error(format!("Failed to upload file: {e}"))),
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Ok(ToolOutput::error(format!(
                "File upload failed ({status}): {body}"
            )));
        }

        let upload_resp: FileUploadResponse = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to parse upload response: {e}"
                )))
            }
        };

        let file_ids: Vec<String> = upload_resp
            .file_infos
            .iter()
            .map(|f| f.id.clone())
            .collect();
        if file_ids.is_empty() {
            return Ok(ToolOutput::error(
                "Upload succeeded but no file IDs returned",
            ));
        }

        debug!(file_ids = ?file_ids, "File uploaded to Mattermost");

        // Step 2: Create a post with the file_ids attached.
        let post_body = PostBodyWithFiles {
            channel_id: self.channel_id.clone(),
            message,
            root_id: self.root_id.clone(),
            file_ids,
        };

        match self
            .api
            .post::<PostBodyWithFiles, PostResponse>("posts", None, &post_body)
            .await
        {
            Ok(_) => Ok(ToolOutput::success("File uploaded and posted successfully")),
            Err(e) => Ok(ToolOutput::error(format!(
                "File uploaded but failed to create post: {e}"
            ))),
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
/// * `server_url` — base URL of the Mattermost server
/// * `auth_token` — bot auth token for API calls
/// * `api` — shared Mattermost client
pub fn build_mattermost_tools(
    channel_id: String,
    post_id: String,
    root_id: Option<String>,
    bot_user_id: String,
    server_url: String,
    auth_token: String,
    api: Arc<Mattermost>,
) -> Vec<Arc<dyn ToolHandler>> {
    vec![
        Arc::new(MattermostReplyHandler {
            channel_id: channel_id.clone(),
            root_id: root_id.clone(),
            api: api.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(MattermostReactHandler {
            post_id,
            bot_user_id,
            api: api.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(MattermostUploadHandler {
            channel_id,
            root_id,
            server_url,
            auth_token,
            api,
        }) as Arc<dyn ToolHandler>,
    ]
}

/// Resolve file content bytes from either a `content` (text) or
/// `content_base64` (base64-encoded binary) parameter.
///
/// Returns `Ok(bytes)` on success, or an error string for
/// `ToolOutput::error()`.
fn resolve_upload_bytes(params: &HashMap<String, Value>) -> Result<Vec<u8>, String> {
    if let Some(b64) = params.get("content_base64").and_then(|v| v.as_str()) {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("Invalid base64 in content_base64: {e}"))
    } else if let Some(text) = params.get("content").and_then(|v| v.as_str()) {
        Ok(text.as_bytes().to_vec())
    } else {
        Err("Either 'content' or 'content_base64' must be provided".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_upload_bytes;
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
        let original = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let encoded = base64::engine::general_purpose::STANDARD.encode(&original);
        let p = params(&[("content_base64", json!(encoded))]);
        let bytes = resolve_upload_bytes(&p).unwrap();
        assert_eq!(bytes, original);
    }

    #[test]
    fn upload_bytes_base64_takes_priority() {
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
        assert!(err.contains("content"), "error should mention content");
    }

    #[test]
    fn upload_bytes_invalid_base64_returns_error() {
        let p = params(&[("content_base64", json!("not-valid-base64!!!"))]);
        let err = resolve_upload_bytes(&p).unwrap_err();
        assert!(err.contains("Invalid base64"));
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
}
