//! Per-turn extension tools for the Matrix interface.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::warn;

use crate::client::MatrixClient;

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
