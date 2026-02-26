//! `ToolExecutor` — dispatches tool calls to registered `ToolHandler` implementations.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use assistant_core::{AssistantConfig, ExecutionContext, SubagentRunner, ToolHandler, ToolOutput};
use assistant_llm::{LlmProvider, ToolSpec};
use assistant_storage::{SkillRegistry, StorageLayer};
use tracing::warn;

pub struct ToolExecutor {
    storage: Arc<StorageLayer>,
    tool_handlers: RwLock<HashMap<String, Arc<dyn ToolHandler>>>,
}

impl ToolExecutor {
    pub fn new(
        storage: Arc<StorageLayer>,
        llm: Arc<dyn LlmProvider>,
        registry: Arc<SkillRegistry>,
        config: Arc<AssistantConfig>,
    ) -> Self {
        let executor = Self {
            storage: storage.clone(),
            tool_handlers: RwLock::new(HashMap::new()),
        };
        executor.register_builtins(llm, registry, config);
        executor
    }

    fn register_builtins(
        &self,
        llm: Arc<dyn LlmProvider>,
        registry: Arc<SkillRegistry>,
        config: Arc<AssistantConfig>,
    ) {
        use crate::builtins::*;
        let storage = self.storage.clone();

        let tools: Vec<Arc<dyn ToolHandler>> = vec![
            // File I/O
            Arc::new(FileReadHandler::new()),
            Arc::new(FileWriteHandler::new()),
            Arc::new(FileEditHandler::new()),
            Arc::new(FileGlobHandler::new()),
            // Shell
            Arc::new(BashHandler::new()),
            // Web
            Arc::new(WebFetchHandler::new()),
            Arc::new(WebSearchHandler::new()),
            // Memory
            Arc::new(MemoryGetHandler::new(config.clone())),
            Arc::new(MemoryAppendHandler::new(config.clone())),
            Arc::new(MemorySearchHandler::new(storage.clone(), llm.clone())),
            // Skills / meta
            Arc::new(ListSkillsHandler::new(registry.clone())),
            Arc::new(LoadSkillHandler::new(registry.clone())),
            Arc::new(SelfAnalyzeHandler::new(storage.clone(), llm, registry)),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
            Arc::new(CancelTaskHandler::new(storage.clone())),
            Arc::new(ListTasksHandler::new(storage.clone())),
        ];

        let mut tool_handlers = self.tool_handlers.write().unwrap();
        for t in tools {
            tool_handlers.insert(t.name().to_string(), t);
        }
    }

    /// Register an ambient tool contributed by an interface (e.g. `slack-post`).
    pub fn register_ambient_tool(&self, handler: Arc<dyn ToolHandler>) {
        self.tool_handlers
            .write()
            .unwrap()
            .insert(handler.name().to_string(), handler);
    }

    /// Inject the subagent runner and register the `agent-spawn` tool.
    ///
    /// This must be called *after* both [`ToolExecutor`] and the
    /// [`SubagentRunner`] implementor (e.g. `Orchestrator`) have been
    /// constructed, because they have a circular dependency at init time.
    pub fn set_subagent_runner(&self, runner: Arc<dyn SubagentRunner>) {
        use crate::builtins::AgentSpawnHandler;
        self.register_ambient_tool(Arc::new(AgentSpawnHandler::new(runner)));
    }

    /// Returns all registered tool handlers.
    pub fn list_tools(&self) -> Vec<Arc<dyn ToolHandler>> {
        self.tool_handlers
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Returns a `ToolSpec` for each registered handler, sorted by name.
    pub fn to_specs(&self) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = self
            .tool_handlers
            .read()
            .unwrap()
            .values()
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                params_schema: t.params_schema(),
                is_mutating: t.is_mutating(),
                requires_confirmation: t.requires_confirmation(),
            })
            .collect();
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }

    /// Returns a `ToolSpec` for each registered handler whose name appears in
    /// `allowed`, sorted by name.  When `allowed` is `None`, all handlers are
    /// included (identical to [`to_specs`]).
    pub fn to_specs_filtered(&self, allowed: &Option<Vec<String>>) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = self
            .tool_handlers
            .read()
            .unwrap()
            .values()
            .filter(|t| match allowed {
                Some(list) => list.iter().any(|a| a == t.name()),
                None => true,
            })
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                params_schema: t.params_schema(),
                is_mutating: t.is_mutating(),
                requires_confirmation: t.requires_confirmation(),
            })
            .collect();
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }

    /// Execute a tool by name with the given parameters.
    ///
    /// If `ctx.allowed_tools` is `Some`, only tools in that list may be
    /// executed; all others are rejected with a non-fatal error.
    pub async fn execute(
        &self,
        name: &str,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        // Enforce the per-context tool allowlist when present.
        if let Some(ref allowed) = ctx.allowed_tools {
            if !allowed.iter().any(|a| a == name) {
                return Ok(ToolOutput::error(format!(
                    "Tool '{}' is not available in this execution context",
                    name
                )));
            }
        }

        // Clone Arc before releasing the read lock to avoid holding it across an await.
        let handler = self.tool_handlers.read().unwrap().get(name).cloned();

        if let Some(handler) = handler {
            // Validate params against the declared JSON Schema before dispatch.
            if let Some(err) = validate_params(name, &handler.params_schema(), &params) {
                return Ok(err);
            }
            return handler.run(params, ctx).await;
        }

        Ok(ToolOutput::error(format!(
            "No tool handler registered for '{}'",
            name
        )))
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Validate `params` against the JSON Schema declared by `schema_json`.
/// Returns `Some(ToolOutput::error(...))` if validation fails, `None` if OK.
fn validate_params(
    name: &str,
    schema_json: &serde_json::Value,
    params: &HashMap<String, serde_json::Value>,
) -> Option<ToolOutput> {
    let params_val =
        serde_json::Value::Object(params.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

    match jsonschema::validate(schema_json, &params_val) {
        Ok(()) => None,
        Err(e) => {
            warn!(tool = %name, error = %e, "Parameter validation failed");
            Some(ToolOutput::error(format!(
                "Invalid parameters for tool '{name}': {e}"
            )))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::Interface;
    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;

    /// Minimal stub handler for testing dispatch and filtering.
    struct StubHandler {
        tool_name: &'static str,
    }

    #[async_trait]
    impl ToolHandler for StubHandler {
        fn name(&self) -> &str {
            self.tool_name
        }
        fn description(&self) -> &str {
            "stub"
        }
        fn params_schema(&self) -> serde_json::Value {
            json!({"type": "object", "properties": {}, "required": []})
        }
        async fn run(
            &self,
            _params: HashMap<String, serde_json::Value>,
            _ctx: &ExecutionContext,
        ) -> Result<ToolOutput> {
            Ok(ToolOutput::success(format!("ok from {}", self.tool_name)))
        }
    }

    async fn make_executor_with_stubs() -> ToolExecutor {
        let storage = Arc::new(
            assistant_storage::StorageLayer::new_in_memory()
                .await
                .unwrap(),
        );
        let executor = ToolExecutor {
            storage,
            tool_handlers: RwLock::new(HashMap::new()),
        };
        executor.register_ambient_tool(Arc::new(StubHandler { tool_name: "alpha" }));
        executor.register_ambient_tool(Arc::new(StubHandler { tool_name: "beta" }));
        executor.register_ambient_tool(Arc::new(StubHandler { tool_name: "gamma" }));
        executor
    }

    fn ctx_unrestricted() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
        }
    }

    fn ctx_restricted(allowed: Vec<&str>) -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: Some(allowed.into_iter().map(String::from).collect()),
            depth: 0,
        }
    }

    // -- execute filtering ---------------------------------------------------

    #[tokio::test]
    async fn execute_unrestricted_allows_all_tools() {
        let executor = make_executor_with_stubs().await;
        let ctx = ctx_unrestricted();

        let out = executor
            .execute("alpha", HashMap::new(), &ctx)
            .await
            .unwrap();
        assert!(out.success, "alpha should succeed");

        let out = executor
            .execute("beta", HashMap::new(), &ctx)
            .await
            .unwrap();
        assert!(out.success, "beta should succeed");
    }

    #[tokio::test]
    async fn execute_restricted_allows_listed_tool() {
        let executor = make_executor_with_stubs().await;
        let ctx = ctx_restricted(vec!["alpha"]);

        let out = executor
            .execute("alpha", HashMap::new(), &ctx)
            .await
            .unwrap();
        assert!(out.success, "alpha is in allowed_tools and should succeed");
    }

    #[tokio::test]
    async fn execute_restricted_rejects_unlisted_tool() {
        let executor = make_executor_with_stubs().await;
        let ctx = ctx_restricted(vec!["alpha"]);

        let out = executor
            .execute("beta", HashMap::new(), &ctx)
            .await
            .unwrap();
        assert!(!out.success, "beta is not in allowed_tools and should fail");
        assert!(
            out.content.contains("not available"),
            "error message should mention unavailability"
        );
    }

    #[tokio::test]
    async fn execute_restricted_empty_list_rejects_all() {
        let executor = make_executor_with_stubs().await;
        let ctx = ctx_restricted(vec![]);

        let out = executor
            .execute("alpha", HashMap::new(), &ctx)
            .await
            .unwrap();
        assert!(!out.success, "empty allowed_tools should reject all tools");
    }

    // -- to_specs_filtered ---------------------------------------------------

    #[tokio::test]
    async fn to_specs_filtered_none_returns_all() {
        let executor = make_executor_with_stubs().await;
        let specs = executor.to_specs_filtered(&None);
        assert_eq!(specs.len(), 3);
    }

    #[tokio::test]
    async fn to_specs_filtered_restricts_to_listed() {
        let executor = make_executor_with_stubs().await;
        let allowed = Some(vec!["alpha".to_string(), "gamma".to_string()]);
        let specs = executor.to_specs_filtered(&allowed);
        let names: Vec<&str> = specs.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "gamma"]);
    }

    #[tokio::test]
    async fn to_specs_filtered_empty_list_returns_empty() {
        let executor = make_executor_with_stubs().await;
        let allowed = Some(vec![]);
        let specs = executor.to_specs_filtered(&allowed);
        assert!(specs.is_empty());
    }
}
