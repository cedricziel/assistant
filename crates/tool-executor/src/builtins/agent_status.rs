//! Builtin handler for the `agent-status` tool — queries subagent lifecycle info.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;
use serde_json::json;

pub struct AgentStatusHandler {
    storage: Arc<StorageLayer>,
}

impl AgentStatusHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl ToolHandler for AgentStatusHandler {
    fn name(&self) -> &str {
        "agent-status"
    }

    fn description(&self) -> &str {
        "Query the status of sub-agents. When called with an agent_id, returns \
         details for that specific agent. When called without arguments, lists \
         all agents spawned in the current conversation."
    }

    fn params_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "Optional ID of a specific agent to query. If omitted, all agents in the current conversation are listed."
                }
            }
        })
    }

    fn is_mutating(&self) -> bool {
        false
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let store = self.storage.agent_store();

        if let Some(agent_id) = params.get("agent_id").and_then(|v| v.as_str()) {
            // Single-agent lookup
            match store.get(agent_id).await? {
                Some(agent) => {
                    let completed_at = agent
                        .completed_at
                        .map(|dt| dt.to_rfc3339())
                        .unwrap_or_else(|| "n/a".to_string());
                    let result = agent.result_summary.as_deref().unwrap_or("n/a");

                    let text = format!(
                        "Agent: {}\n\
                         Status: {}\n\
                         Task: {}\n\
                         Depth: {}\n\
                         Parent conversation: {}\n\
                         Conversation: {}\n\
                         Created: {}\n\
                         Completed: {}\n\
                         Result: {}",
                        agent.id,
                        agent.status.as_str(),
                        agent.task,
                        agent.depth,
                        agent.parent_conversation_id,
                        agent.conversation_id,
                        agent.created_at.to_rfc3339(),
                        completed_at,
                        result,
                    );
                    Ok(ToolOutput::success(text))
                }
                None => Ok(ToolOutput::error(format!(
                    "No agent found with id '{agent_id}'"
                ))),
            }
        } else {
            // List all agents in the current conversation
            let conversation_id = ctx.conversation_id.to_string();
            let agents = store.list_by_parent_conversation(&conversation_id).await?;

            if agents.is_empty() {
                return Ok(ToolOutput::success(
                    "No sub-agents have been spawned in this conversation.".to_string(),
                ));
            }

            let mut lines = Vec::with_capacity(agents.len() + 1);
            lines.push(format!(
                "Found {} sub-agent(s) in this conversation:\n",
                agents.len()
            ));

            for agent in &agents {
                let completed_at = agent
                    .completed_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "n/a".to_string());
                let result_preview = agent.result_summary.as_deref().unwrap_or("n/a");
                // Truncate long results for the list view
                let result_display = if result_preview.len() > 120 {
                    format!("{}...", &result_preview[..120])
                } else {
                    result_preview.to_string()
                };

                lines.push(format!(
                    "- **{}** ({})\n  Task: {}\n  Depth: {} | Created: {} | Completed: {}\n  Result: {}",
                    agent.id,
                    agent.status.as_str(),
                    agent.task,
                    agent.depth,
                    agent.created_at.to_rfc3339(),
                    completed_at,
                    result_display,
                ));
            }

            Ok(ToolOutput::success(lines.join("\n")))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use assistant_storage::{AgentStore, StorageLayer};
    use uuid::Uuid;

    fn ctx_with_conversation(conversation_id: Uuid) -> ExecutionContext {
        ExecutionContext {
            conversation_id,
            turn: 1,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
        }
    }

    async fn setup() -> (Arc<StorageLayer>, AgentStatusHandler, AgentStore) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let handler = AgentStatusHandler::new(storage.clone());
        let agent_store = storage.agent_store();
        (storage, handler, agent_store)
    }

    fn params(pairs: &[(&str, serde_json::Value)]) -> HashMap<String, serde_json::Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn no_agents_returns_empty_message() {
        let (_storage, handler, _store) = setup().await;
        let ctx = ctx_with_conversation(Uuid::new_v4());
        let out = handler.run(params(&[]), &ctx).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("No sub-agents"));
    }

    #[tokio::test]
    async fn lists_agents_in_conversation() {
        let (_storage, handler, store) = setup().await;
        let conv_id = Uuid::new_v4();
        let conv_str = conv_id.to_string();

        store
            .create("agent-1", None, &conv_str, "child-conv-1", "task one", 1)
            .await
            .unwrap();
        store
            .create("agent-2", None, &conv_str, "child-conv-2", "task two", 1)
            .await
            .unwrap();

        let ctx = ctx_with_conversation(conv_id);
        let out = handler.run(params(&[]), &ctx).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("agent-1"));
        assert!(out.content.contains("agent-2"));
        assert!(out.content.contains("task one"));
        assert!(out.content.contains("task two"));
        assert!(out.content.contains("2 sub-agent(s)"));
    }

    #[tokio::test]
    async fn get_specific_agent_by_id() {
        let (_storage, handler, store) = setup().await;
        let conv_id = Uuid::new_v4();

        store
            .create(
                "agent-abc",
                None,
                &conv_id.to_string(),
                "child-conv",
                "specific task",
                2,
            )
            .await
            .unwrap();

        let ctx = ctx_with_conversation(conv_id);
        let out = handler
            .run(params(&[("agent_id", json!("agent-abc"))]), &ctx)
            .await
            .unwrap();
        assert!(out.success);
        assert!(out.content.contains("agent-abc"));
        assert!(out.content.contains("specific task"));
        assert!(out.content.contains("running"));
        assert!(out.content.contains("Depth: 2"));
    }

    #[tokio::test]
    async fn get_nonexistent_agent_returns_error() {
        let (_storage, handler, _store) = setup().await;
        let ctx = ctx_with_conversation(Uuid::new_v4());

        let out = handler
            .run(params(&[("agent_id", json!("no-such-agent"))]), &ctx)
            .await
            .unwrap();
        assert!(!out.success);
        assert!(out.content.contains("no-such-agent"));
    }

    #[tokio::test]
    async fn completed_agent_shows_result() {
        let (_storage, handler, store) = setup().await;
        let conv_id = Uuid::new_v4();

        store
            .create(
                "agent-done",
                None,
                &conv_id.to_string(),
                "child-conv",
                "finished task",
                1,
            )
            .await
            .unwrap();

        use assistant_storage::AgentStatus;
        store
            .complete("agent-done", AgentStatus::Completed, Some("All done!"))
            .await
            .unwrap();

        let ctx = ctx_with_conversation(conv_id);
        let out = handler
            .run(params(&[("agent_id", json!("agent-done"))]), &ctx)
            .await
            .unwrap();
        assert!(out.success);
        assert!(out.content.contains("completed"));
        assert!(out.content.contains("All done!"));
    }

    #[tokio::test]
    async fn does_not_list_agents_from_other_conversations() {
        let (_storage, handler, store) = setup().await;
        let my_conv = Uuid::new_v4();
        let other_conv = Uuid::new_v4();

        store
            .create(
                "agent-mine",
                None,
                &my_conv.to_string(),
                "child-conv",
                "my task",
                1,
            )
            .await
            .unwrap();
        store
            .create(
                "agent-other",
                None,
                &other_conv.to_string(),
                "child-conv-2",
                "other task",
                1,
            )
            .await
            .unwrap();

        let ctx = ctx_with_conversation(my_conv);
        let out = handler.run(params(&[]), &ctx).await.unwrap();
        assert!(out.success);
        assert!(out.content.contains("agent-mine"));
        assert!(
            !out.content.contains("agent-other"),
            "should not see agents from other conversations"
        );
    }
}
