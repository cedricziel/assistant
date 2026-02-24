//! `ToolExecutor` — dispatches tool calls to registered `ToolHandler` implementations.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use assistant_core::{AssistantConfig, ExecutionContext, ToolHandler, ToolOutput};
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
            Arc::new(MemorySearchHandler::new(storage.clone(), llm.clone())),
            // Skills / meta
            Arc::new(ListSkillsHandler::new(registry.clone())),
            Arc::new(LoadSkillHandler::new(registry.clone())),
            Arc::new(SelfAnalyzeHandler::new(storage.clone(), llm, registry)),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
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

    /// Execute a tool by name with the given parameters.
    pub async fn execute(
        &self,
        name: &str,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
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
