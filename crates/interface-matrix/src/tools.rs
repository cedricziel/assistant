//! Per-turn extension tools for the Matrix interface.
//!
//! These tools are injected into the orchestrator via `register_extensions`
//! so the worker dispatches to `run_turn_with_tools`.  The LLM calls
//! `matrix-reply` to post its response back into the originating room.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use matrix_sdk::room::Room;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use serde_json::{json, Value};

// ── MatrixReplyHandler ────────────────────────────────────────────────────────

struct MatrixReplyHandler {
    room: Room,
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

        match self
            .room
            .send(RoomMessageEventContent::text_plain(text))
            .await
        {
            Ok(_) => Ok(ToolOutput::success("Message posted successfully")),
            Err(e) => Ok(ToolOutput::error(format!("Failed to post message: {e}"))),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Build per-turn Matrix extension tools for the given joined room.
///
/// The returned list contains `matrix-reply` which the LLM calls to post its
/// response.  Pass the list to `Orchestrator::register_extensions` before
/// calling `submit_turn`.
pub fn build_matrix_tools(room: Room) -> Vec<Arc<dyn ToolHandler>> {
    vec![Arc::new(MatrixReplyHandler { room })]
}
