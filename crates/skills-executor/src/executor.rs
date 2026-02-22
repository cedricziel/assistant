use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    AssistantConfig, ExecutionContext, SkillDef, SkillHandler, SkillOutput, SkillTier,
};
use assistant_llm::LlmClient;
use assistant_storage::{SkillRegistry, StorageLayer};

pub struct SkillExecutor {
    storage: Arc<StorageLayer>,
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
        let handlers: Vec<Arc<dyn SkillHandler>> = vec![
            Arc::new(MemoryReadHandler::new(storage.clone())),
            Arc::new(MemoryWriteHandler::new(storage.clone())),
            Arc::new(MemorySearchHandler::new(storage.clone())),
            Arc::new(BashHandler::new()),
            Arc::new(WebFetchHandler::new()),
            Arc::new(ShellExecHandler::new()),
            Arc::new(ListSkillsHandler::new(registry.clone())),
            Arc::new(SkillFileReadHandler::new(registry.clone())),
            Arc::new(SelfAnalyzeHandler::new(storage.clone(), llm, registry)),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
            Arc::new(MemorySaveHandler::new(config.clone())),
            Arc::new(SoulUpdateHandler::new(config)),
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
        if let Some(handler) = self.builtin_handlers.get(&def.name) {
            handler.execute(def, params, ctx).await
        } else {
            Ok(SkillOutput::error(format!(
                "No builtin handler registered for skill '{}'",
                def.name
            )))
        }
    }
}
