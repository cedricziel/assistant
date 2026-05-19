//! Per-turn extension tools for the Mattermost interface.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ToolHandler, ToolOutput, resolve_upload_bytes, types::conversation::ExecutionContext,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{debug, warn};

use super::client::MattermostClient;

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;
    use assistant_core::ToolHandler;
    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use serde_json::{Value, json};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: uuid::Uuid::new_v4(),
            agent_id: "default".into(),
            turn: 0,
            interface: Interface::Mattermost,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn params(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    async fn make_client(server: &MockServer) -> Arc<MattermostClient> {
        let client = MattermostClient::new("https://placeholder", "test-token")
            .unwrap()
            .with_base_url(&server.uri());
        Arc::new(client)
    }

    fn find_tool<'a>(tools: &'a [Arc<dyn ToolHandler>], name: &str) -> &'a Arc<dyn ToolHandler> {
        tools
            .iter()
            .find(|t| t.name() == name)
            .expect("tool present")
    }

    #[test]
    fn build_mattermost_tools_returns_three_tools() {
        // We don't care about HTTP here, just shape.
        let client = Arc::new(MattermostClient::new("https://placeholder", "t").unwrap());
        let tools = build_mattermost_tools(
            "C1".into(),
            "P1".into(),
            Some("R1".into()),
            "U_BOT".into(),
            client,
        );
        assert_eq!(tools.len(), 3);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"mattermost-reply"));
        assert!(names.contains(&"mattermost-react"));
        assert!(names.contains(&"mattermost-upload"));
    }

    #[tokio::test]
    async fn reply_missing_text_returns_error_output() {
        let client = Arc::new(MattermostClient::new("https://placeholder", "t").unwrap());
        let tools = build_mattermost_tools("C1".into(), "P1".into(), None, "U_BOT".into(), client);
        let reply = find_tool(&tools, "mattermost-reply");
        let out = reply.run(params(&[]), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("text"));
    }

    #[tokio::test]
    async fn reply_posts_message_via_client() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/posts"))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({ "id": "post-1", "channel_id": "C1" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let tools = build_mattermost_tools(
            "C1".into(),
            "P1".into(),
            Some("root-1".into()),
            "U_BOT".into(),
            make_client(&server).await,
        );
        let reply = find_tool(&tools, "mattermost-reply");
        let out = reply
            .run(params(&[("text", json!("hello world"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn react_missing_emoji_returns_error_output() {
        let client = Arc::new(MattermostClient::new("https://placeholder", "t").unwrap());
        let tools = build_mattermost_tools("C1".into(), "P1".into(), None, "U_BOT".into(), client);
        let react = find_tool(&tools, "mattermost-react");
        let out = react.run(params(&[]), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("emoji"));
    }

    #[tokio::test]
    async fn react_adds_reaction_via_client() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/reactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let tools = build_mattermost_tools(
            "C1".into(),
            "post-x".into(),
            None,
            "U_BOT".into(),
            make_client(&server).await,
        );
        let react = find_tool(&tools, "mattermost-react");
        let out = react
            .run(params(&[("emoji", json!("tada"))]), &ctx())
            .await
            .unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn upload_missing_filename_returns_error_output() {
        let client = Arc::new(MattermostClient::new("https://placeholder", "t").unwrap());
        let tools = build_mattermost_tools("C1".into(), "P1".into(), None, "U_BOT".into(), client);
        let upload = find_tool(&tools, "mattermost-upload");
        let out = upload.run(params(&[]), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("filename"));
    }
}
