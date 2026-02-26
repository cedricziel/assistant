//! Builtin handler for the `agent-terminate` tool — cancels a running sub-agent.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SubagentRunner, ToolHandler, ToolOutput};
use async_trait::async_trait;
use serde_json::json;
use tracing::info;

pub struct AgentTerminateHandler {
    runner: Arc<dyn SubagentRunner>,
}

impl AgentTerminateHandler {
    pub fn new(runner: Arc<dyn SubagentRunner>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl ToolHandler for AgentTerminateHandler {
    fn name(&self) -> &str {
        "agent-terminate"
    }

    fn description(&self) -> &str {
        "Cancel a running sub-agent by its ID. The agent will stop at the \
         next cancellation check point (between iterations or tool executions)."
    }

    fn params_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "The ID of the sub-agent to cancel."
                }
            },
            "required": ["agent_id"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok(ToolOutput::error("Missing required parameter 'agent_id'")),
        };

        info!(agent_id, "Requesting sub-agent termination");

        match self.runner.cancel_agent(agent_id).await {
            Ok(true) => Ok(ToolOutput::success(format!(
                "Cancellation signal sent to agent '{agent_id}'. \
                 The agent will stop at the next check point."
            ))),
            Ok(false) => Ok(ToolOutput::error(format!(
                "No running agent found with id '{agent_id}'. \
                 It may have already completed or does not exist."
            ))),
            Err(e) => Ok(ToolOutput::error(format!(
                "Failed to cancel agent '{agent_id}': {e}"
            ))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::{AgentReport, AgentReportStatus, AgentSpawn, Interface};
    use std::sync::Mutex;
    use uuid::Uuid;

    /// A mock SubagentRunner that tracks cancel_agent calls.
    struct MockRunner {
        cancel_calls: Mutex<Vec<String>>,
        cancel_result: bool,
    }

    impl MockRunner {
        fn new(cancel_result: bool) -> Self {
            Self {
                cancel_calls: Mutex::new(Vec::new()),
                cancel_result,
            }
        }

        fn cancel_call_count(&self) -> usize {
            self.cancel_calls.lock().unwrap().len()
        }

        fn last_cancel_id(&self) -> Option<String> {
            self.cancel_calls.lock().unwrap().last().cloned()
        }
    }

    #[async_trait]
    impl SubagentRunner for MockRunner {
        async fn run_subagent(
            &self,
            _spawn: AgentSpawn,
            _parent_depth: u32,
        ) -> Result<AgentReport> {
            Ok(AgentReport {
                status: AgentReportStatus::Completed,
                content: "unused".into(),
                data: None,
            })
        }

        async fn cancel_agent(&self, agent_id: &str) -> Result<bool> {
            self.cancel_calls.lock().unwrap().push(agent_id.to_string());
            Ok(self.cancel_result)
        }
    }

    fn test_ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
        }
    }

    #[tokio::test]
    async fn terminate_running_agent_returns_success() {
        let runner = Arc::new(MockRunner::new(true));
        let handler = AgentTerminateHandler::new(runner.clone());
        let mut params = HashMap::new();
        params.insert("agent_id".into(), json!("subagent-123"));

        let out = handler.run(params, &test_ctx()).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("Cancellation signal sent"));
        assert_eq!(runner.cancel_call_count(), 1);
        assert_eq!(runner.last_cancel_id().unwrap(), "subagent-123");
    }

    #[tokio::test]
    async fn terminate_nonexistent_agent_returns_error() {
        let runner = Arc::new(MockRunner::new(false));
        let handler = AgentTerminateHandler::new(runner.clone());
        let mut params = HashMap::new();
        params.insert("agent_id".into(), json!("no-such-agent"));

        let out = handler.run(params, &test_ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("No running agent found"));
        assert_eq!(runner.cancel_call_count(), 1);
    }

    #[tokio::test]
    async fn missing_agent_id_returns_error() {
        let runner = Arc::new(MockRunner::new(true));
        let handler = AgentTerminateHandler::new(runner.clone());
        let params = HashMap::new();

        let out = handler.run(params, &test_ctx()).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("agent_id"));
        assert_eq!(runner.cancel_call_count(), 0);
    }
}
