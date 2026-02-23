use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use assistant_core::{
    AssistantConfig, ExecutionContext, SkillDef, SkillHandler, SkillOutput, SkillSource, SkillTier,
    ToolHandler,
};
use assistant_llm::LlmProvider;
use assistant_storage::{SkillRegistry, StorageLayer};
use tracing::warn;

pub struct SkillExecutor {
    storage: Arc<StorageLayer>,
    /// Primitive, self-describing tools (file-read, bash, etc.)
    tool_handlers: RwLock<HashMap<String, Arc<dyn ToolHandler>>>,
    /// SKILL.md-backed builtin handlers (memory-get, self-analyze, etc.)
    builtin_handlers: RwLock<HashMap<String, Arc<dyn SkillHandler>>>,
    /// Ambient skill definitions contributed by interfaces (e.g. slack-post).
    ambient_defs: RwLock<Vec<SkillDef>>,
}

impl SkillExecutor {
    pub fn new(
        storage: Arc<StorageLayer>,
        llm: Arc<dyn LlmProvider>,
        registry: Arc<SkillRegistry>,
        config: Arc<AssistantConfig>,
    ) -> Self {
        let executor = Self {
            storage: storage.clone(),
            tool_handlers: RwLock::new(HashMap::new()),
            builtin_handlers: RwLock::new(HashMap::new()),
            ambient_defs: RwLock::new(Vec::new()),
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

        // Primitive tools — implement ToolHandler
        let tools: Vec<Arc<dyn ToolHandler>> = vec![
            // File I/O
            Arc::new(FileReadHandler::new()),
            Arc::new(FileWriteHandler::new()),
            Arc::new(FileEditHandler::new()),
            Arc::new(FileGlobHandler::new()),
            // Shell
            Arc::new(BashHandler::new()),
            Arc::new(ShellExecHandler::new()),
            // Web
            Arc::new(WebFetchHandler::new()),
            Arc::new(WebSearchHandler::new()),
        ];
        {
            let mut tool_handlers = self.tool_handlers.write().unwrap();
            for t in tools {
                tool_handlers.insert(t.name().to_string(), t);
            }
        }

        // SKILL.md-backed builtin handlers — implement SkillHandler
        let handlers: Vec<Arc<dyn SkillHandler>> = vec![
            // Memory read
            Arc::new(MemoryGetHandler::new(config.clone())),
            Arc::new(MemorySearchHandler::new(storage.clone(), llm.clone())),
            // Heartbeat
            Arc::new(HeartbeatReadHandler::new(config.clone())),
            Arc::new(HeartbeatUpdateHandler::new(config.clone())),
            // Skills / meta
            Arc::new(ListSkillsHandler::new(registry.clone())),
            Arc::new(SkillFileReadHandler::new(registry.clone())),
            Arc::new(SelfAnalyzeHandler::new(storage.clone(), llm, registry)),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
        ];
        {
            let mut builtin_handlers = self.builtin_handlers.write().unwrap();
            for h in handlers {
                builtin_handlers.insert(h.skill_name().to_string(), h);
            }
        }
    }

    /// Register an ambient skill contributed by an interface (e.g. `slack-post`).
    ///
    /// The skill definition is added to [`synthetic_skill_defs`] so it appears
    /// in the orchestrator's LLM prompt. The handler is registered alongside
    /// the builtin handlers so it can be dispatched normally.
    pub fn register_ambient_skill(&self, def: SkillDef, handler: Arc<dyn SkillHandler>) {
        let name = def.name.clone();
        self.builtin_handlers.write().unwrap().insert(name, handler);
        self.ambient_defs.write().unwrap().push(def);
    }

    pub async fn execute(
        &self,
        def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        match &def.tier {
            SkillTier::Builtin => self.execute_builtin(def, params, ctx).await,
            SkillTier::Script { entrypoint } => {
                crate::script_executor::run_script(entrypoint, &params, ctx).await
            }
            SkillTier::Prompt => crate::prompt_executor::run_prompt(def, &params, ctx).await,
            SkillTier::Wasm { .. } => {
                // WASM execution via extism is future work — return a clear error
                Ok(SkillOutput::error(
                    "WASM skill execution is not yet implemented",
                ))
            }
        }
    }

    async fn execute_builtin(
        &self,
        def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        // Clone the Arc before releasing the read lock to avoid holding the
        // lock guard across an await point.
        let tool = self.tool_handlers.read().unwrap().get(&def.name).cloned();
        if let Some(tool) = tool {
            // Validate params against the declared JSON Schema before dispatch.
            if let Some(err) = validate_params(&def.name, &tool.params_schema(), &params) {
                return Ok(err);
            }
            return tool.run(params, ctx).await;
        }
        // Then check SKILL.md-backed handlers
        let handler = self
            .builtin_handlers
            .read()
            .unwrap()
            .get(&def.name)
            .cloned();
        if let Some(handler) = handler {
            return handler.execute(def, params, ctx).await;
        }
        Ok(SkillOutput::error(format!(
            "No builtin handler registered for skill '{}'",
            def.name
        )))
    }

    /// Returns `SkillDef` objects synthesised from self-describing tool handlers
    /// and registered ambient skills.
    pub fn synthetic_skill_defs(&self) -> Vec<SkillDef> {
        let mut defs: Vec<SkillDef> = self
            .tool_handlers
            .read()
            .unwrap()
            .values()
            .map(|tool| skill_def_from_tool(tool.as_ref()))
            .collect();
        // Include ambient defs contributed by interfaces (e.g. slack-post).
        defs.extend(self.ambient_defs.read().unwrap().iter().cloned());
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Look up a single synthetic skill def by name. Returns `None` if the
    /// tool handler does not exist.
    pub fn get_synthetic_def(&self, name: &str) -> Option<SkillDef> {
        // Check primitive tool handlers first.
        if let Some(def) = self
            .tool_handlers
            .read()
            .unwrap()
            .get(name)
            .map(|tool| skill_def_from_tool(tool.as_ref()))
        {
            return Some(def);
        }
        // Fall back to ambient defs contributed by interfaces (e.g. slack-post).
        self.ambient_defs
            .read()
            .unwrap()
            .iter()
            .find(|d| d.name == name)
            .cloned()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Build a synthetic `SkillDef` from a `ToolHandler`'s self-description.
fn skill_def_from_tool(tool: &dyn ToolHandler) -> SkillDef {
    SkillDef {
        name: tool.name().to_string(),
        description: tool.description().to_string(),
        license: None,
        compatibility: None,
        allowed_tools: vec![],
        metadata: HashMap::new(),
        body: String::new(),
        dir: std::path::PathBuf::new(),
        tier: SkillTier::Builtin,
        mutating: tool.is_mutating(),
        confirmation_required: tool.requires_confirmation(),
        source: SkillSource::Builtin,
    }
}

/// Validate `params` against the JSON Schema declared by `schema_json`.
/// Returns `Some(SkillOutput::error(...))` if validation fails, `None` if OK.
fn validate_params(
    name: &str,
    schema_json: &serde_json::Value,
    params: &HashMap<String, serde_json::Value>,
) -> Option<SkillOutput> {
    let params_val =
        serde_json::Value::Object(params.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

    match jsonschema::validate(schema_json, &params_val) {
        Ok(()) => None,
        Err(e) => {
            warn!(skill = %name, error = %e, "Parameter validation failed");
            Some(SkillOutput::error(format!(
                "Invalid parameters for skill '{name}': {e}"
            )))
        }
    }
}
