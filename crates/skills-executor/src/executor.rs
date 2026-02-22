use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    AssistantConfig, ExecutionContext, SkillDef, SkillHandler, SkillOutput, SkillSource, SkillTier,
    ToolHandler,
};
use assistant_llm::LlmClient;
use assistant_storage::{SkillRegistry, StorageLayer};

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
            Arc::new(MemoryPatchHandler::new(config)),
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
        let mut defs = Vec::new();
        for tool in self.tool_handlers.values() {
            let mut metadata = HashMap::new();
            let schema = tool.params_schema();
            if let Ok(json_str) = serde_json::to_string(&schema) {
                metadata.insert("params".to_string(), json_str);
            }
            defs.push(SkillDef {
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
            });
        }
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Look up a single synthetic skill def by name. Returns `None` if the
    /// tool handler does not exist.
    pub fn get_synthetic_def(&self, name: &str) -> Option<SkillDef> {
        let tool = self.tool_handlers.get(name)?;
        let mut metadata = HashMap::new();
        let schema = tool.params_schema();
        if let Ok(json_str) = serde_json::to_string(&schema) {
            metadata.insert("params".to_string(), json_str);
        }
        Some(SkillDef {
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
        })
    }
}
