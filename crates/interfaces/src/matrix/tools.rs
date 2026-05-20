//! Per-turn extension tools for the Matrix interface.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ToolHandler, ToolOutput, types::conversation::ExecutionContext};
use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::warn;

use super::client::MatrixClient;

// ── matrix-reply ──────────────────────────────────────────────────────────────

struct MatrixReplyHandler {
    room_id: String,
    client: Arc<MatrixClient>,
}

#[async_trait]
impl ToolHandler for MatrixReplyHandler {
    fn name(&self) -> &str {
        "matrix-reply"
    }

    fn description(&self) -> &str {
        "Post a reply message in the current Matrix room. \
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
        match self.client.send_message(&self.room_id, text).await {
            Ok(()) => Ok(ToolOutput::success("Message posted")),
            Err(e) => {
                warn!(error = %e, "matrix-reply failed");
                Ok(ToolOutput::error(format!("Failed: {e}")))
            }
        }
    }
}

// ── Public factory ────────────────────────────────────────────────────────────

pub fn build_matrix_tools(room_id: String, client: Arc<MatrixClient>) -> Vec<Arc<dyn ToolHandler>> {
    vec![Arc::new(MatrixReplyHandler { room_id, client })]
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use uuid::Uuid;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
            turn: 0,
            interface: Interface::Matrix,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn client_at(server: &MockServer) -> Arc<MatrixClient> {
        Arc::new(MatrixClient::new_with_token(server.uri(), "tok", "@bot:example.com").unwrap())
    }

    #[tokio::test]
    async fn build_returns_single_reply_handler() {
        let server = MockServer::start().await;
        let tools = build_matrix_tools("!room:example.com".to_string(), client_at(&server));
        assert_eq!(tools.len(), 1);
        let h = &tools[0];
        assert_eq!(h.name(), "matrix-reply");
        assert!(h.is_mutating());
        assert!(h.description().contains("Matrix"));
        assert!(h.params_schema().get("required").is_some());
    }

    #[tokio::test]
    async fn run_posts_message_when_text_present() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id": "$1"})),
            )
            .mount(&server)
            .await;
        let tools = build_matrix_tools("!room:example.com".to_string(), client_at(&server));
        let mut params = HashMap::new();
        params.insert("text".into(), json!("hello"));
        let out = tools[0].run(params, &ctx()).await.unwrap();
        assert!(out.success);
    }

    #[tokio::test]
    async fn run_errors_on_missing_text() {
        let server = MockServer::start().await;
        let tools = build_matrix_tools("!room:example.com".to_string(), client_at(&server));
        let out = tools[0].run(HashMap::new(), &ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("text"));
    }

    #[tokio::test]
    async fn run_reports_failure_on_server_error() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({"errcode": "M_UNKNOWN"})),
            )
            .mount(&server)
            .await;
        let tools = build_matrix_tools("!room:example.com".to_string(), client_at(&server));
        let mut params = HashMap::new();
        params.insert("text".into(), json!("hi"));
        let out = tools[0].run(params, &ctx()).await.unwrap();
        assert!(!out.success);
    }
}
