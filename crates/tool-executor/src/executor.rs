//! `ToolExecutor` — dispatches tool calls to registered `ToolHandler` implementations.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use assistant_core::types::conversation::ExecutionContext;
use assistant_core::{
    LlmProvider, SubagentRunner, ToolHandler, ToolOutput, ToolSpec, types::agent::OrgConfig,
};
use assistant_storage::{SkillRegistry, SkillStatsProvider, StorageLayer};
use tracing::warn;

use crate::overlay::{ToolOverlay, spec_of};

pub struct ToolExecutor {
    storage: Arc<StorageLayer>,
    tool_handlers: RwLock<HashMap<String, Arc<dyn ToolHandler>>>,
}

impl ToolExecutor {
    pub fn new(
        storage: Arc<StorageLayer>,
        llm: Arc<dyn LlmProvider>,
        registry: Arc<SkillRegistry>,
        config: Arc<OrgConfig>,
    ) -> Self {
        let stats_provider: Arc<dyn SkillStatsProvider> = Arc::new(storage.skill_execution_store());
        let executor = Self {
            storage: storage.clone(),
            tool_handlers: RwLock::new(HashMap::new()),
        };
        executor.register_builtins(llm, registry, config, stats_provider);
        executor
    }

    fn register_builtins(
        &self,
        llm: Arc<dyn LlmProvider>,
        registry: Arc<SkillRegistry>,
        config: Arc<OrgConfig>,
        stats_provider: Arc<dyn SkillStatsProvider>,
    ) {
        use crate::builtins::*;
        let storage = self.storage.clone();
        let extra_roots = config.permissions.workspace_roots.clone();

        let tools: Vec<Arc<dyn ToolHandler>> = vec![
            // File I/O
            Arc::new(FileReadHandler::new().with_extra_roots(extra_roots.clone())),
            Arc::new(FileWriteHandler::new().with_extra_roots(extra_roots.clone())),
            Arc::new(FileEditHandler::new().with_extra_roots(extra_roots.clone())),
            Arc::new(FileGlobHandler::new().with_extra_roots(extra_roots)),
            // Shell
            Arc::new(
                BashHandler::new()
                    .with_limits(config.permissions.limits.clone())
                    .with_extra_roots(config.permissions.workspace_roots.clone()),
            ),
            Arc::new(ProcessHandler::new().with_limits(config.permissions.limits.clone())),
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
            Arc::new(SelfAnalyzeHandler::new(
                storage.clone(),
                llm,
                registry,
                stats_provider,
            )),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
            Arc::new(CancelTaskHandler::new(storage.clone())),
            Arc::new(ListTasksHandler::new(storage.clone())),
            // Subagent lifecycle
            Arc::new(AgentStatusHandler::new(storage.clone())),
        ];

        let mut tool_handlers = self
            .tool_handlers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for t in tools {
            tool_handlers.insert(t.name().to_string(), t);
        }
    }

    /// Register an ambient tool contributed by an interface (e.g. `slack-post`).
    pub fn register_ambient_tool(&self, handler: Arc<dyn ToolHandler>) {
        self.tool_handlers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(handler.name().to_string(), handler);
    }

    /// Register the `voice-response` tool.
    ///
    /// Call after construction when both a TTS provider and audio store are
    /// available (i.e. when the web UI has voice configured).
    pub fn register_voice_response(
        &self,
        tts: Arc<dyn assistant_transcription::TtsProvider>,
        store: Arc<assistant_transcription::AudioStore>,
    ) {
        use crate::builtins::VoiceResponseHandler;
        let handler = Arc::new(VoiceResponseHandler::new(tts, store));
        self.tool_handlers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(handler.name().to_string(), handler);
    }

    /// Inject the subagent runner and register subagent tools.
    ///
    /// Registers `agent-spawn` and `agent-terminate`.  This must be called
    /// *after* both [`ToolExecutor`] and the [`SubagentRunner`] implementor
    /// (e.g. `Orchestrator`) have been constructed, because they have a
    /// circular dependency at init time.
    pub fn set_subagent_runner(&self, runner: Arc<dyn SubagentRunner>) {
        use crate::builtins::{AgentSpawnHandler, AgentTerminateHandler};
        self.register_ambient_tool(Arc::new(AgentSpawnHandler::new(runner.clone())));
        self.register_ambient_tool(Arc::new(AgentTerminateHandler::new(runner)));
    }

    /// Remove all tools whose names start with `prefix`.
    ///
    /// Used by the MCP client manager to clear stale tools from a server before
    /// re-registering refreshed ones (e.g. after a `tools/list_changed`
    /// notification).
    pub fn unregister_tools_by_prefix(&self, prefix: &str) {
        self.tool_handlers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|name, _| !name.starts_with(prefix));
    }

    /// Returns all registered tool handlers.
    pub fn list_tools(&self) -> Vec<Arc<dyn ToolHandler>> {
        self.tool_handlers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .cloned()
            .collect()
    }

    /// Returns a `ToolSpec` for each registered handler, sorted by name.
    pub fn to_specs(&self) -> Vec<ToolSpec> {
        let mut specs: Vec<ToolSpec> = self
            .tool_handlers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .map(spec_of)
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
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter(|t| match allowed {
                Some(list) => list.iter().any(|a| a == t.name()),
                None => true,
            })
            .map(spec_of)
            .collect();
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        specs
    }

    /// The registry's specs plus the turn's `overlay`, sorted by name. An
    /// overlay tool replaces a registered one of the same name.
    pub fn to_specs_with(&self, overlay: &ToolOverlay) -> Vec<ToolSpec> {
        let mut specs = self.to_specs();
        if overlay.is_empty() {
            return specs;
        }
        specs.retain(|spec| overlay.get(&spec.name).is_none());
        specs.extend(overlay.specs());
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
        self.execute_with(&ToolOverlay::default(), name, params, ctx)
            .await
    }

    /// [`execute`](Self::execute), looking in the turn's `overlay` before the
    /// registry.
    pub async fn execute_with(
        &self,
        overlay: &ToolOverlay,
        name: &str,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        // Enforce the per-context tool allowlist when present.
        if let Some(ref allowed) = ctx.allowed_tools
            && !allowed.iter().any(|a| a == name)
        {
            return Ok(ToolOutput::error(format!(
                "Tool '{}' is not available in this execution context",
                name
            )));
        }

        // Clone Arc before releasing the read lock to avoid holding it across an await.
        let handler = overlay.get(name).cloned().or_else(|| {
            self.tool_handlers
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(name)
                .cloned()
        });

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

#[async_trait::async_trait]
impl assistant_core::tool::ToolDispatcher for ToolExecutor {
    fn to_specs(&self) -> Vec<ToolSpec> {
        ToolExecutor::to_specs(self)
    }

    async fn execute(
        &self,
        name: &str,
        params: std::collections::HashMap<String, serde_json::Value>,
        ctx: &assistant_core::types::conversation::ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        ToolExecutor::execute(self, name, params, ctx).await
    }

    fn is_mutating(&self, name: &str) -> bool {
        let handlers = self
            .tool_handlers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        handlers.get(name).map(|h| h.is_mutating()).unwrap_or(false)
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
    use assistant_core::types::conversation::Interface;
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
            agent_id: "default".to_string(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    fn ctx_restricted(allowed: Vec<&str>) -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "default".to_string(),
            turn: 0,
            interface: Interface::Cli,
            interactive: false,
            allowed_tools: Some(allowed.into_iter().map(String::from).collect()),
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
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

    // -- unregister_tools_by_prefix ------------------------------------------

    #[tokio::test]
    async fn unregister_tools_by_prefix_removes_matching() {
        let executor = make_executor_with_stubs().await;
        executor.register_ambient_tool(Arc::new(StubHandler {
            tool_name: "mcp__gh__issue",
        }));
        executor.register_ambient_tool(Arc::new(StubHandler {
            tool_name: "mcp__gh__pr",
        }));
        executor.register_ambient_tool(Arc::new(StubHandler {
            tool_name: "mcp__db__query",
        }));

        assert_eq!(executor.list_tools().len(), 6);
        executor.unregister_tools_by_prefix("mcp__gh__");
        let remaining: Vec<String> = executor
            .list_tools()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        assert_eq!(remaining.len(), 4);
        assert!(!remaining.contains(&"mcp__gh__issue".to_string()));
        assert!(!remaining.contains(&"mcp__gh__pr".to_string()));
        assert!(remaining.contains(&"mcp__db__query".to_string()));
    }

    #[tokio::test]
    async fn unregister_tools_by_prefix_noop_when_no_match() {
        let executor = make_executor_with_stubs().await;
        executor.unregister_tools_by_prefix("mcp__nonexistent__");
        assert_eq!(executor.list_tools().len(), 3);
    }

    // -- turn overlay ---------------------------------------------------------

    fn overlay_of(names: &[&'static str]) -> ToolOverlay {
        ToolOverlay::new(
            names
                .iter()
                .map(|n| Arc::new(StubHandler { tool_name: n }) as Arc<dyn ToolHandler>)
                .collect(),
        )
    }

    #[tokio::test]
    async fn overlay_specs_are_added_to_the_turn_list_only() {
        let executor = make_executor_with_stubs().await;
        let overlay = overlay_of(&["mcp__user_linear__create"]);

        let names: Vec<String> = executor
            .to_specs_with(&overlay)
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(
            names,
            vec!["alpha", "beta", "gamma", "mcp__user_linear__create"]
        );
        assert_eq!(
            executor.to_specs().len(),
            3,
            "the registry itself must not learn the overlay's tools"
        );
    }

    #[tokio::test]
    async fn overlay_tool_runs_without_being_registered() {
        let executor = make_executor_with_stubs().await;
        let overlay = overlay_of(&["mcp__user_linear__create"]);

        let out = executor
            .execute_with(
                &overlay,
                "mcp__user_linear__create",
                HashMap::new(),
                &ctx_unrestricted(),
            )
            .await
            .unwrap();
        assert!(out.success);

        let without = executor
            .execute(
                "mcp__user_linear__create",
                HashMap::new(),
                &ctx_unrestricted(),
            )
            .await
            .unwrap();
        assert!(!without.success, "a plain execute must not see the overlay");
    }

    #[tokio::test]
    async fn overlay_is_consulted_before_the_registry() {
        let executor = make_executor_with_stubs().await;
        let overlay = ToolOverlay::new(vec![
            Arc::new(StubHandler { tool_name: "alpha" }) as Arc<dyn ToolHandler>
        ]);
        assert_eq!(
            executor.to_specs_with(&overlay).len(),
            3,
            "a shadowed name is listed once"
        );
    }

    #[tokio::test]
    async fn overlay_respects_the_allowed_tools_list() {
        let executor = make_executor_with_stubs().await;
        let overlay = overlay_of(&["mcp__user_linear__create"]);

        let out = executor
            .execute_with(
                &overlay,
                "mcp__user_linear__create",
                HashMap::new(),
                &ctx_restricted(vec!["alpha"]),
            )
            .await
            .unwrap();
        assert!(!out.success);
    }

    #[tokio::test]
    async fn concurrent_turns_never_see_each_others_overlay() {
        let executor = Arc::new(make_executor_with_stubs().await);
        let turn = |tool: &'static str, other: &'static str| {
            let executor = executor.clone();
            async move {
                let overlay = overlay_of(&[tool]);
                tokio::task::yield_now().await;
                let specs: Vec<String> = executor
                    .to_specs_with(&overlay)
                    .into_iter()
                    .map(|s| s.name)
                    .collect();
                let own = executor
                    .execute_with(&overlay, tool, HashMap::new(), &ctx_unrestricted())
                    .await
                    .unwrap();
                let foreign = executor
                    .execute_with(&overlay, other, HashMap::new(), &ctx_unrestricted())
                    .await
                    .unwrap();
                (specs, own.success, foreign.success)
            }
        };

        let (u, v) = tokio::join!(
            turn("mcp__user_a__tool", "mcp__user_b__tool"),
            turn("mcp__user_b__tool", "mcp__user_a__tool"),
        );

        assert!(u.0.contains(&"mcp__user_a__tool".to_string()));
        assert!(!u.0.contains(&"mcp__user_b__tool".to_string()));
        assert!(u.1 && !u.2, "U runs its own tool and not V's");
        assert!(v.0.contains(&"mcp__user_b__tool".to_string()));
        assert!(!v.0.contains(&"mcp__user_a__tool".to_string()));
        assert!(v.1 && !v.2, "V runs its own tool and not U's");
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
