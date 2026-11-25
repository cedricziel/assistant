//! Action executor contract and the built-in `assistant_turn` executor.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

/// Input passed to action executors.
#[derive(Debug, Clone)]
pub struct WorkflowActionInput {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub node_id: String,
    pub config: Value,
    pub trigger_payload: Value,
}

/// Action execution output consumed by graph traversal.
#[derive(Debug, Clone)]
pub struct WorkflowActionResult {
    pub outcome_label: String,
    pub note: Option<String>,
    pub output: Option<Value>,
}

impl WorkflowActionResult {
    /// Success-style result.
    pub fn success(note: impl Into<String>) -> Self {
        Self {
            outcome_label: "success".to_string(),
            note: Some(note.into()),
            output: None,
        }
    }

    /// Failure-style result.
    pub fn failure(note: impl Into<String>) -> Self {
        Self {
            outcome_label: "failure".to_string(),
            note: Some(note.into()),
            output: None,
        }
    }

    /// Attach structured step output.
    pub fn with_output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }
}

/// Executor plugin interface for workflow action node types.
#[async_trait]
pub trait WorkflowActionExecutor: Send + Sync {
    /// Action type this executor handles, e.g. `assistant_turn`, `http_request`.
    fn action_type(&self) -> &'static str;

    /// Executes one action node.
    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult>;
}

/// Client abstraction to submit assistant turns from workflows.
#[async_trait]
pub trait AssistantTurnClient: Send + Sync {
    /// Submit a prompt and return the assistant answer text.
    async fn submit_turn(&self, prompt: &str, conversation_id: Uuid) -> Result<String>;
}

/// Built-in action executor for `assistant_turn` nodes.
pub struct AssistantTurnActionExecutor {
    client: Arc<dyn AssistantTurnClient>,
}

impl AssistantTurnActionExecutor {
    /// Construct from any [`AssistantTurnClient`] implementation.
    pub fn new(client: Arc<dyn AssistantTurnClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl WorkflowActionExecutor for AssistantTurnActionExecutor {
    fn action_type(&self) -> &'static str {
        "assistant_turn"
    }

    async fn execute(&self, input: WorkflowActionInput) -> Result<WorkflowActionResult> {
        let prompt = input
            .config
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("Continue workflow action");

        let answer = self.client.submit_turn(prompt, input.run_id).await?;
        Ok(WorkflowActionResult::success(format!(
            "assistant_turn success: {} chars",
            answer.chars().count()
        )))
    }
}
