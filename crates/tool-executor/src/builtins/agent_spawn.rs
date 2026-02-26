//! Builtin handler for the `agent-spawn` tool — delegates a task to
//! an isolated sub-agent and blocks until it completes.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    AgentReportStatus, AgentSpawn, ExecutionContext, SubagentRunner, ToolHandler, ToolOutput,
};
use async_trait::async_trait;
use serde_json::json;
use tracing::info;

pub struct AgentSpawnHandler {
    runner: Arc<dyn SubagentRunner>,
}

impl AgentSpawnHandler {
    pub fn new(runner: Arc<dyn SubagentRunner>) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl ToolHandler for AgentSpawnHandler {
    fn name(&self) -> &str {
        "agent-spawn"
    }

    fn description(&self) -> &str {
        "Spawn an isolated sub-agent to perform a task. The tool blocks until \
         the sub-agent completes and returns its result. Use this to delegate \
         self-contained subtasks that benefit from a clean context."
    }

    fn params_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "What the sub-agent should accomplish. Be specific and self-contained."
                },
                "system_prompt": {
                    "type": "string",
                    "description": "Optional system prompt override for the sub-agent. If omitted, the default system prompt is used."
                },
                "allowed_tools": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of tool names the sub-agent may use. If omitted, all tools are available."
                }
            },
            "required": ["task"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let task = match params.get("task").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return Ok(ToolOutput::error("Missing required parameter 'task'")),
        };

        let system_prompt = params
            .get("system_prompt")
            .and_then(|v| v.as_str())
            .map(String::from);

        let allowed_tools: Vec<String> = params
            .get("allowed_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let agent_id = format!("subagent-{}", uuid::Uuid::new_v4());

        info!(
            agent_id = %agent_id,
            task = %task,
            parent_depth = ctx.depth,
            allowed_tools = ?allowed_tools,
            "Spawning sub-agent"
        );

        let spawn = AgentSpawn {
            agent_id: agent_id.clone(),
            task,
            system_prompt,
            model: None,
            allowed_tools,
        };

        let report = self.runner.run_subagent(spawn, ctx.depth).await?;

        match report.status {
            AgentReportStatus::Completed => {
                let mut output = ToolOutput::success(&report.content);
                if let Some(data) = report.data {
                    output = output.with_data(data);
                }
                Ok(output)
            }
            AgentReportStatus::Failed => Ok(ToolOutput::error(format!(
                "Sub-agent '{}' failed: {}",
                agent_id, report.content
            ))),
            AgentReportStatus::Cancelled => Ok(ToolOutput::error(format!(
                "Sub-agent '{}' was cancelled: {}",
                agent_id, report.content
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::{AgentReport, Interface};
    use std::sync::Mutex;
    use uuid::Uuid;

    /// A mock SubagentRunner that records calls and returns a fixed report.
    struct MockRunner {
        report: AgentReport,
        calls: Mutex<Vec<(AgentSpawn, u32)>>,
    }

    impl MockRunner {
        fn new(report: AgentReport) -> Self {
            Self {
                report,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }

        fn last_spawn(&self) -> Option<AgentSpawn> {
            self.calls.lock().unwrap().last().map(|(s, _)| s.clone())
        }
    }

    #[async_trait]
    impl SubagentRunner for MockRunner {
        async fn run_subagent(&self, spawn: AgentSpawn, parent_depth: u32) -> Result<AgentReport> {
            self.calls.lock().unwrap().push((spawn, parent_depth));
            Ok(self.report.clone())
        }
    }

    fn test_ctx(depth: u32) -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth,
        }
    }

    #[tokio::test]
    async fn spawn_completed_returns_success() {
        let runner = Arc::new(MockRunner::new(AgentReport {
            status: AgentReportStatus::Completed,
            content: "The answer is 42.".into(),
            data: None,
        }));
        let handler = AgentSpawnHandler::new(runner.clone());
        let mut params = HashMap::new();
        params.insert("task".into(), json!("What is the meaning of life?"));

        let out = handler.run(params, &test_ctx(0)).await.unwrap();
        assert!(out.success);
        assert_eq!(out.content, "The answer is 42.");
        assert_eq!(runner.call_count(), 1);
    }

    #[tokio::test]
    async fn spawn_failed_returns_error() {
        let runner = Arc::new(MockRunner::new(AgentReport {
            status: AgentReportStatus::Failed,
            content: "LLM timed out".into(),
            data: None,
        }));
        let handler = AgentSpawnHandler::new(runner);
        let mut params = HashMap::new();
        params.insert("task".into(), json!("do something"));

        let out = handler.run(params, &test_ctx(0)).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("failed"));
    }

    #[tokio::test]
    async fn spawn_passes_depth_from_context() {
        let runner = Arc::new(MockRunner::new(AgentReport {
            status: AgentReportStatus::Completed,
            content: "done".into(),
            data: None,
        }));
        let handler = AgentSpawnHandler::new(runner.clone());
        let mut params = HashMap::new();
        params.insert("task".into(), json!("nested task"));

        handler.run(params, &test_ctx(3)).await.unwrap();
        let calls = runner.calls.lock().unwrap();
        assert_eq!(calls[0].1, 3, "parent_depth should match ctx.depth");
    }

    #[tokio::test]
    async fn spawn_passes_allowed_tools() {
        let runner = Arc::new(MockRunner::new(AgentReport {
            status: AgentReportStatus::Completed,
            content: "done".into(),
            data: None,
        }));
        let handler = AgentSpawnHandler::new(runner.clone());
        let mut params = HashMap::new();
        params.insert("task".into(), json!("restricted task"));
        params.insert("allowed_tools".into(), json!(["file-read", "web-fetch"]));

        handler.run(params, &test_ctx(0)).await.unwrap();
        let spawn = runner.last_spawn().unwrap();
        assert_eq!(spawn.allowed_tools, vec!["file-read", "web-fetch"]);
    }

    #[tokio::test]
    async fn spawn_passes_system_prompt() {
        let runner = Arc::new(MockRunner::new(AgentReport {
            status: AgentReportStatus::Completed,
            content: "done".into(),
            data: None,
        }));
        let handler = AgentSpawnHandler::new(runner.clone());
        let mut params = HashMap::new();
        params.insert("task".into(), json!("custom prompt task"));
        params.insert(
            "system_prompt".into(),
            json!("You are a research assistant."),
        );

        handler.run(params, &test_ctx(0)).await.unwrap();
        let spawn = runner.last_spawn().unwrap();
        assert_eq!(
            spawn.system_prompt.unwrap(),
            "You are a research assistant."
        );
    }

    #[tokio::test]
    async fn missing_task_returns_error() {
        let runner = Arc::new(MockRunner::new(AgentReport {
            status: AgentReportStatus::Completed,
            content: "done".into(),
            data: None,
        }));
        let handler = AgentSpawnHandler::new(runner.clone());
        let params = HashMap::new();

        let out = handler.run(params, &test_ctx(0)).await.unwrap();
        assert!(!out.success);
        assert!(out.content.contains("task"));
        assert_eq!(runner.call_count(), 0, "runner should not be called");
    }
}
