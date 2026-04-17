//! Per-turn extension tools for the Mattermost interface.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput, resolve_upload_bytes};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::client::MattermostClient;

// ── mattermost-reply ──────────────────────────────────────────────────────────

struct MattermostReplyHandler {
    channel_id: String,
    root_id: Option<String>,
    client: Arc<MattermostClient>,
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
                "text": { "type": "string", "description": "Message text to post" }
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
            Some(t) => t,
            None => return Ok(ToolOutput::error("Missing 'text'")),
        };
        match self
            .client
            .create_post(&self.channel_id, text, self.root_id.as_deref())
            .await
        {
            Ok(_) => Ok(ToolOutput::success("Message posted")),
            Err(e) => {
                warn!(error = %e, "mattermost-reply failed");
                Ok(ToolOutput::error(format!("Failed: {e}")))
            }
        }
    }
}

// ── mattermost-react ──────────────────────────────────────────────────────────

struct MattermostReactHandler {
    post_id: String,
    bot_user_id: String,
    client: Arc<MattermostClient>,
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
                "emoji": { "type": "string", "description": "Emoji name without colons, e.g. thumbsup" }
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
            Some(e) => e,
            None => return Ok(ToolOutput::error("Missing 'emoji'")),
        };
        match self
            .client
            .add_reaction(&self.bot_user_id, &self.post_id, emoji)
            .await
        {
            Ok(()) => Ok(ToolOutput::success("Reaction added")),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("exists") || msg.contains("already") {
                    Ok(ToolOutput::success("Reaction already present"))
                } else {
                    warn!(error = %e, "mattermost-react failed");
                    Ok(ToolOutput::error(format!("Failed: {e}")))
                }
            }
        }
    }
}

// ── mattermost-upload ─────────────────────────────────────────────────────────

struct MattermostUploadHandler {
    channel_id: String,
    root_id: Option<String>,
    client: Arc<MattermostClient>,
}

#[async_trait]
impl ToolHandler for MattermostUploadHandler {
    fn name(&self) -> &str {
        "mattermost-upload"
    }

    fn description(&self) -> &str {
        "Upload a file to the current Mattermost channel. \
         For text content, set `content`. For binary files, set `path` to an absolute path."
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["filename"],
            "properties": {
                "path":     { "type": "string" },
                "content":  { "type": "string" },
                "filename": { "type": "string" },
                "message":  { "type": "string" }
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
            None => return Ok(ToolOutput::error("Missing 'filename'")),
        };
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let bytes = match resolve_upload_bytes(&params) {
            Ok(b) => b,
            Err(msg) => return Ok(ToolOutput::error(msg)),
        };

        let file_ids = match self
            .client
            .upload_file(&self.channel_id, &filename, bytes)
            .await
        {
            Ok(ids) => ids,
            Err(e) => return Ok(ToolOutput::error(format!("Upload failed: {e}"))),
        };

        if file_ids.is_empty() {
            return Ok(ToolOutput::error(
                "Upload succeeded but no file IDs returned",
            ));
        }

        debug!(file_ids = ?file_ids, "File uploaded to Mattermost");

        match self
            .client
            .create_post_with_files(
                &self.channel_id,
                &message,
                self.root_id.as_deref(),
                file_ids,
            )
            .await
        {
            Ok(()) => Ok(ToolOutput::success("File uploaded and posted")),
            Err(e) => Ok(ToolOutput::error(format!("Post with file failed: {e}"))),
        }
    }
}

// ── Public factory ────────────────────────────────────────────────────────────

pub fn build_mattermost_tools(
    channel_id: String,
    post_id: String,
    root_id: Option<String>,
    bot_user_id: String,
    client: Arc<MattermostClient>,
) -> Vec<Arc<dyn ToolHandler>> {
    vec![
        Arc::new(MattermostReplyHandler {
            channel_id: channel_id.clone(),
            root_id: root_id.clone(),
            client: client.clone(),
        }) as Arc<dyn ToolHandler>,
        Arc::new(MattermostReactHandler {
            post_id,
            bot_user_id,
            client: client.clone(),
        }),
        Arc::new(MattermostUploadHandler {
            channel_id,
            root_id,
            client,
        }),
    ]
}
