use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput, SkillTier};
use assistant_storage::StorageLayer;

pub struct SkillExecutor {
    storage: Arc<StorageLayer>,
    builtin_handlers: HashMap<String, Arc<dyn SkillHandler>>,
}

impl SkillExecutor {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        let mut executor = Self {
            storage: storage.clone(),
            builtin_handlers: HashMap::new(),
        };
        // Register all builtin handlers
        executor.register_builtins();
        executor
    }

    fn register_builtins(&mut self) {
        use crate::builtins::*;
        let storage = self.storage.clone();
        let handlers: Vec<Arc<dyn SkillHandler>> = vec![
            Arc::new(MemoryReadHandler::new(storage.clone())),
            Arc::new(MemoryWriteHandler::new(storage.clone())),
            Arc::new(MemorySearchHandler::new(storage.clone())),
            Arc::new(WebFetchHandler::new()),
            Arc::new(ShellExecHandler::new()),
            Arc::new(SelfAnalyzeHandler::new(storage.clone())),
            Arc::new(ScheduleTaskHandler::new(storage.clone())),
        ];
        for h in handlers {
            self.builtin_handlers.insert(h.skill_name().to_string(), h);
        }
    }

    /// Register a `ListSkillsHandler` that already holds a reference to the skill registry.
    /// This must be called separately after the registry is available.
    pub fn register_list_skills_handler(&mut self, handler: Arc<dyn SkillHandler>) {
        self.builtin_handlers
            .insert(handler.skill_name().to_string(), handler);
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
