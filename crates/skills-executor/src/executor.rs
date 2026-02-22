use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    AssistantConfig, ExecutionContext, SkillDef, SkillHandler, SkillOutput, SkillSource, SkillTier,
    ToolHandler,
};
use assistant_llm::LlmClient;
use assistant_storage::{SkillRegistry, StorageLayer};
use tracing::warn;

pub struct SkillExecutor {
    storage: Arc<StorageLayer>,
    /// Primitive, self-describing tools (file-read, bash, etc.)
    tool_handlers: HashMap<String, Arc<dyn ToolHandler>>,
    /// SKILL.md-backed builtin handlers (memory-read, self-analyze, etc.)
    builtin_handlers: HashMap<String, Arc<dyn SkillHandler>>,
}

impl SkillExecutor {
    pub fn new(
        storage: Arc<StorageLayer>,
        llm: Arc<LlmClient>,
        registry: Arc<SkillRegistry>,
        config: Arc<AssistantConfig>,
    ) -> Self {
        let mut executor = Self {
            storage: storage.clone(),
            tool_handlers: HashMap::new(),
            builtin_handlers: HashMap::new(),
        };
        executor.register_builtins(llm, registry, config);
        executor
    }

    fn register_builtins(
        &mut self,
        llm: Arc<LlmClient>,
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
        for t in tools {
            self.tool_handlers.insert(t.name().to_string(), t);
        }

        // SKILL.md-backed builtin handlers — implement SkillHandler
        let handlers: Vec<Arc<dyn SkillHandler>> = vec![
            // Memory / soul
            Arc::new(MemoryReadHandler::new(storage.clone())),
            Arc::new(MemoryWriteHandler::new(storage.clone())),
            Arc::new(MemorySearchHandler::new(storage.clone())),
            Arc::new(MemorySaveHandler::new(config.clone())),
            Arc::new(SoulUpdateHandler::new(config.clone())),
            Arc::new(MemoryPatchHandler::new(config.clone())),
            // Heartbeat
            Arc::new(HeartbeatReadHandler::new(config.clone())),
            Arc::new(HeartbeatUpdateHandler::new(config.clone())),
            // Skills / meta
            Arc::new(ListSkillsHandler::new(registry.clone())),
            Arc::new(SkillFileReadHandler::new(registry.clone())),
            Arc::new(SelfAnalyzeHandler::new(storage.clone(), llm, registry)),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
        ];
        for h in handlers {
            self.builtin_handlers.insert(h.skill_name().to_string(), h);
        }
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
        // Check tool handlers first (primitive tools)
        if let Some(tool) = self.tool_handlers.get(&def.name) {
            // Validate params against the declared JSON Schema before dispatch.
            if let Some(err) = validate_params(&def.name, &tool.params_schema(), &params) {
                return Ok(err);
            }
            return tool.run(params, ctx).await;
        }
        // Then check SKILL.md-backed handlers
        if let Some(handler) = self.builtin_handlers.get(&def.name) {
            return handler.execute(def, params, ctx).await;
        }
        Ok(SkillOutput::error(format!(
            "No builtin handler registered for skill '{}'",
            def.name
        )))
    }

    /// Returns `SkillDef` objects synthesised from self-describing tool handlers.
    pub fn synthetic_skill_defs(&self) -> Vec<SkillDef> {
        let mut defs: Vec<SkillDef> = self
            .tool_handlers
            .values()
            .map(|tool| skill_def_from_tool(tool.as_ref()))
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Look up a single synthetic skill def by name. Returns `None` if the
    /// tool handler does not exist.
    pub fn get_synthetic_def(&self, name: &str) -> Option<SkillDef> {
        self.tool_handlers
            .get(name)
            .map(|tool| skill_def_from_tool(tool.as_ref()))
    }
}

/// Build a [`SkillDef`] from a [`ToolHandler`], including both `params` and
/// `output_schema` in the metadata map.
fn skill_def_from_tool(tool: &dyn ToolHandler) -> SkillDef {
    let mut metadata = HashMap::new();
    if let Ok(s) = serde_json::to_string(&tool.params_schema()) {
        metadata.insert("params".to_string(), s);
    }
    if let Some(out) = tool.output_schema() {
        if let Ok(s) = serde_json::to_string(&out) {
            metadata.insert("output_schema".to_string(), s);
        }
    }
    SkillDef {
        name: tool.name().to_string(),
        description: tool.description().to_string(),
        license: None,
        compatibility: None,
        allowed_tools: vec![],
        metadata,
        body: String::new(),
        dir: std::path::PathBuf::new(),
        tier: SkillTier::Builtin,
        mutating: tool.is_mutating(),
        confirmation_required: tool.requires_confirmation(),
        source: SkillSource::Builtin,
    }
}

/// Validate `params` against the tool's JSON Schema.
///
/// Returns `Some(SkillOutput::error(...))` if validation fails so the caller
/// can short-circuit immediately. Returns `None` if params are valid or the
/// schema cannot be compiled (non-fatal — we proceed and let the handler
/// catch any issues itself).
fn validate_params(
    tool_name: &str,
    schema: &serde_json::Value,
    params: &HashMap<String, serde_json::Value>,
) -> Option<SkillOutput> {
    let instance =
        serde_json::Value::Object(params.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    match jsonschema::validator_for(schema) {
        Err(e) => {
            warn!("Failed to compile params schema for '{tool_name}': {e}");
            None
        }
        Ok(validator) => {
            let errors: Vec<String> = validator
                .iter_errors(&instance)
                .map(|e| e.to_string())
                .collect();
            if errors.is_empty() {
                None
            } else {
                Some(SkillOutput::error(format!(
                    "Invalid parameters for '{tool_name}': {}",
                    errors.join("; ")
                )))
            }
        }
    }
}
