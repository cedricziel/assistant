//! Per-space runtimes.
//!
//! A [`SpaceRuntime`] is everything a turn needs, bound to one space's
//! database: the skill registry, tool executor, message bus and orchestrator.
//! [`SpaceRuntimes`] builds them lazily per `(org, space)` and caches them, so
//! a turn submitted for space B reads and writes B's `space.db` and never
//! touches another space.
//!
//! A space gets its LLM provider and settings from the org it belongs to
//! ([`OrgContexts`]); the confirmation callback and MCP tools are shared by
//! every space. Only the storage-bound pieces are per space.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use anyhow::{Context, Result};
use tokio::sync::OnceCell;
use tracing::{info, warn};

use assistant_core::tool::{ToolHandler, TurnToolSource};
use assistant_core::types::agent::OrgConfig;
use assistant_core::{LlmProvider, MessageBus};
use assistant_skills::{SkillDef, SkillSource};
use assistant_storage::{
    OrgPoolFactory, PersonaStore as _, ScopedMessageBus, StorageLayer, registry::SkillRegistry,
};
use assistant_tool_executor::ToolExecutor;
use assistant_transcription::{AudioStore, TtsProvider};

use crate::orchestrator::{ConfirmationCallback, Orchestrator};
use crate::org_context::{OrgContext, OrgContexts};

type SpaceKey = (String, String);
type Slot = Arc<OnceCell<Arc<SpaceRuntime>>>;

/// The orchestrator stack of one space.
pub struct SpaceRuntime {
    pub org_slug: String,
    pub space_slug: String,
    /// The settings of the org this space belongs to.
    pub config: Arc<OrgConfig>,
    /// The LLM client of the org this space belongs to.
    pub llm: Arc<dyn LlmProvider>,
    pub storage: Arc<StorageLayer>,
    pub registry: Arc<SkillRegistry>,
    pub executor: Arc<ToolExecutor>,
    pub orchestrator: Arc<Orchestrator>,
    pub bus: Arc<dyn MessageBus>,
}

impl SpaceRuntime {
    /// Assemble a runtime from parts that already exist (tests, embedders).
    pub fn from_parts(
        org_slug: impl Into<String>,
        space_slug: impl Into<String>,
        org: OrgContext,
        storage: Arc<StorageLayer>,
        registry: Arc<SkillRegistry>,
        executor: Arc<ToolExecutor>,
        orchestrator: Arc<Orchestrator>,
    ) -> Self {
        let bus = orchestrator.bus().clone();
        Self {
            org_slug: org_slug.into(),
            space_slug: space_slug.into(),
            config: org.config,
            llm: org.llm,
            storage,
            registry,
            executor,
            orchestrator,
            bus,
        }
    }
}

/// Builds and caches one [`SpaceRuntime`] per `(org, space)`.
pub struct SpaceRuntimes {
    pools: Arc<OrgPoolFactory>,
    orgs: Arc<OrgContexts>,
    confirmation: Option<Arc<dyn ConfirmationCallback>>,
    audio_store: Option<Arc<AudioStore>>,
    voice: Option<Arc<dyn TtsProvider>>,
    shared_bus: Option<Arc<dyn MessageBus>>,
    submit_timeout_secs: Option<u64>,
    turn_tool_source: Option<Arc<dyn TurnToolSource>>,
    ambient_tools: RwLock<Vec<Arc<dyn ToolHandler>>>,
    /// Skills each external MCP server publishes, keyed by server name.
    ambient_skills: RwLock<HashMap<String, Vec<SkillDef>>>,
    slots: Mutex<HashMap<SpaceKey, Slot>>,
    builtins_synced: AtomicBool,
}

impl SpaceRuntimes {
    pub fn new(
        pools: Arc<OrgPoolFactory>,
        orgs: Arc<OrgContexts>,
        confirmation: Option<Arc<dyn ConfirmationCallback>>,
    ) -> Self {
        Self {
            pools,
            orgs,
            confirmation,
            audio_store: None,
            voice: None,
            shared_bus: None,
            submit_timeout_secs: None,
            turn_tool_source: None,
            ambient_tools: RwLock::new(Vec::new()),
            ambient_skills: RwLock::new(HashMap::new()),
            slots: Mutex::new(HashMap::new()),
            builtins_synced: AtomicBool::new(false),
        }
    }

    /// Share one audio store between every space's orchestrator.
    pub fn with_audio_store(mut self, store: Arc<AudioStore>) -> Self {
        self.audio_store = Some(store);
        self
    }

    /// Register the `voice-response` tool in every space (needs an audio store).
    pub fn with_voice_response(mut self, tts: Arc<dyn TtsProvider>) -> Self {
        self.voice = Some(tts);
        self
    }

    /// Route every space's turns through one broker (NATS), namespaced per
    /// space. Without it each space uses the SQLite bus in its own database.
    pub fn with_shared_bus(mut self, bus: Arc<dyn MessageBus>) -> Self {
        self.shared_bus = Some(bus);
        self
    }

    /// Override the `submit_turn` timeout, which otherwise comes from the
    /// persona or defaults to three hours.
    pub fn with_submit_timeout(mut self, secs: u64) -> Self {
        self.submit_timeout_secs = Some(secs);
        self
    }

    /// Let every space's turns ask `source` for the tools only that turn may
    /// use. Unlike [`register_ambient_tool`](Self::register_ambient_tool), what
    /// it returns never enters a registry shared between turns.
    pub fn with_turn_tool_source(mut self, source: Arc<dyn TurnToolSource>) -> Self {
        self.turn_tool_source = Some(source);
        self
    }

    pub fn pools(&self) -> &Arc<OrgPoolFactory> {
        &self.pools
    }

    /// The per-org settings and LLM clients the spaces are built from.
    pub fn orgs(&self) -> &Arc<OrgContexts> {
        &self.orgs
    }

    /// The runtime of `(org_slug, space_slug)`, built on first use.
    ///
    /// Concurrent first calls build it once. Slugs are validated by the pool
    /// factory before they reach the filesystem.
    pub async fn get(&self, org_slug: &str, space_slug: &str) -> Result<Arc<SpaceRuntime>> {
        let slot = {
            let mut slots = self.slots.lock().unwrap_or_else(PoisonError::into_inner);
            slots
                .entry((org_slug.to_owned(), space_slug.to_owned()))
                .or_default()
                .clone()
        };
        let runtime = slot
            .get_or_try_init(|| self.build(org_slug, space_slug))
            .await?;
        Ok(runtime.clone())
    }

    /// Serve `runtime` for its `(org, space)` instead of building one.
    pub fn insert(&self, runtime: Arc<SpaceRuntime>) {
        let key = (runtime.org_slug.clone(), runtime.space_slug.clone());
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, Arc::new(OnceCell::new_with(Some(runtime))));
    }

    /// Forget a space's runtime (and its pool), e.g. after the space is deleted.
    pub fn evict(&self, org_slug: &str, space_slug: &str) {
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&(org_slug.to_owned(), space_slug.to_owned()));
        self.pools.evict_space(org_slug, space_slug);
    }

    /// Every runtime built so far.
    pub fn loaded(&self) -> Vec<Arc<SpaceRuntime>> {
        self.slots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .filter_map(|slot| slot.get().cloned())
            .collect()
    }

    /// Register an ambient tool (an MCP tool, say) in every space, including
    /// spaces built later.
    pub fn register_ambient_tool(&self, handler: Arc<dyn ToolHandler>) {
        {
            let mut tools = self
                .ambient_tools
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            tools.retain(|t| t.name() != handler.name());
            tools.push(handler.clone());
        }
        for runtime in self.loaded() {
            runtime.executor.register_ambient_tool(handler.clone());
        }
    }

    /// Remove every ambient tool whose name starts with `prefix`, everywhere.
    pub fn unregister_tools_by_prefix(&self, prefix: &str) {
        self.ambient_tools
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|t| !t.name().starts_with(prefix));
        for runtime in self.loaded() {
            runtime.executor.unregister_tools_by_prefix(prefix);
        }
    }

    /// Replace the skills `server` publishes in every space, including spaces
    /// opened later. An empty list removes them.
    pub async fn set_ambient_skills(&self, server: &str, skills: Vec<SkillDef>) {
        {
            let mut ambient = self
                .ambient_skills
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            if skills.is_empty() {
                ambient.remove(server);
            } else {
                ambient.insert(server.to_owned(), skills.clone());
            }
        }
        for runtime in self.loaded() {
            runtime
                .registry
                .set_mcp_skills(server, skills.clone())
                .await;
        }
    }

    async fn build(&self, org_slug: &str, space_slug: &str) -> Result<Arc<SpaceRuntime>> {
        let OrgContext { config, llm } = self.orgs.get(org_slug)?;
        let storage = self.pools.space_storage(org_slug, space_slug).await?;
        let agent_id = config.agent.id.clone();
        storage.persona_store().ensure_exists(&agent_id).await?;

        let registry = Arc::new(self.load_registry(&storage, &config).await?);
        let ambient_skills = self
            .ambient_skills
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for (server, skills) in ambient_skills {
            registry.set_mcp_skills(&server, skills).await;
        }
        let executor = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            config.clone(),
        ));
        let bus: Arc<dyn MessageBus> = match &self.shared_bus {
            Some(shared) => Arc::new(ScopedMessageBus::new(
                shared.clone(),
                &format!("{org_slug}.{space_slug}"),
            )),
            None => Arc::new(storage.message_bus()),
        };

        let persona_timeout = storage
            .persona_store()
            .get(&agent_id)
            .await
            .ok()
            .flatten()
            .and_then(|p| p.turn_timeout_secs);
        let mut orchestrator = Orchestrator::new(
            llm.clone(),
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus.clone(),
            &config,
        );
        if let Some(callback) = &self.confirmation {
            orchestrator = orchestrator.with_confirmation_callback(callback.clone());
        }
        if let Some(source) = &self.turn_tool_source {
            orchestrator = orchestrator.with_turn_tool_source(source.clone(), org_slug, space_slug);
        }
        if let Some(store) = &self.audio_store {
            orchestrator = orchestrator.with_audio_store(store.clone());
        }
        if let Some(secs) = self.submit_timeout_secs.or(persona_timeout) {
            orchestrator = orchestrator.with_submit_timeout(secs);
        }
        let orchestrator = Arc::new(orchestrator);
        executor.set_subagent_runner(orchestrator.clone());

        let ambient = self
            .ambient_tools
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        for handler in ambient {
            executor.register_ambient_tool(handler);
        }
        if let (Some(tts), Some(store)) = (&self.voice, &self.audio_store) {
            executor.register_voice_response(tts.clone(), store.clone());
        }

        info!(org = org_slug, space = space_slug, "built space runtime");
        Ok(Arc::new(SpaceRuntime {
            org_slug: org_slug.to_owned(),
            space_slug: space_slug.to_owned(),
            config,
            llm,
            storage,
            registry,
            executor,
            orchestrator,
            bus,
        }))
    }

    async fn load_registry(
        &self,
        storage: &StorageLayer,
        config: &OrgConfig,
    ) -> Result<SkillRegistry> {
        let registry = SkillRegistry::new(storage.pool.clone())
            .await
            .context("Failed to create skill registry")?
            .with_user_skills_dir(self.pools.base_dir().join("skills"));
        registry
            .load_embedded()
            .await
            .context("Failed to load embedded builtin skills")?;
        self.sync_builtins_once(&registry);

        let project_root = std::env::current_dir().ok();
        let dirs = crate::bootstrap::skill_dirs_in(
            Some(self.pools.base_dir()),
            config,
            project_root.as_deref(),
        );
        let dirs: Vec<(&Path, SkillSource)> =
            dirs.iter().map(|(p, s)| (p.as_path(), s.clone())).collect();
        registry
            .load_from_dirs(&dirs)
            .await
            .context("Failed to load skills from directories")?;
        Ok(registry)
    }

    /// Keep `{base}/skills` in step with the embedded skills, once per process:
    /// the directory is shared by every space.
    fn sync_builtins_once(&self, registry: &SkillRegistry) {
        if self.builtins_synced.swap(true, Ordering::SeqCst) {
            return;
        }
        let target = self.pools.base_dir().join("skills");
        match registry.sync_builtins_to_disk(&target) {
            Ok(updated) if !updated.is_empty() => {
                info!(
                    "Synced {} built-in skill(s) to disk: {}",
                    updated.len(),
                    updated.join(", ")
                );
            }
            Err(e) => warn!("Failed to sync built-in skills to disk: {e}"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use uuid::Uuid;

    use assistant_core::auth::AuthContext;
    use assistant_core::types::agent::OrgConfig;
    use assistant_core::types::conversation::Interface;
    use assistant_llm_provider::scripted::ScriptedLlmProvider;
    use assistant_storage::{ConversationStore as _, OrgPoolFactory, SqliteConversationStore};

    use super::*;

    fn runtimes(base: &std::path::Path) -> Arc<SpaceRuntimes> {
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        Arc::new(
            SpaceRuntimes::new(
                Arc::new(OrgPoolFactory::new(base)),
                Arc::new(OrgContexts::fixed(
                    config,
                    Arc::new(ScriptedLlmProvider::new()),
                )),
                None,
            )
            .with_submit_timeout(10),
        )
    }

    #[tokio::test]
    async fn concurrent_first_calls_build_one_runtime_per_space() {
        let tmp = tempfile::tempdir().unwrap();
        let runtimes = runtimes(tmp.path());

        let (a1, a2) = tokio::join!(runtimes.get("acme", "alpha"), runtimes.get("acme", "alpha"));
        let (a1, a2) = (a1.unwrap(), a2.unwrap());
        assert!(Arc::ptr_eq(&a1, &a2), "one runtime per (org, space)");

        let beta = runtimes.get("acme", "beta").await.unwrap();
        assert!(!Arc::ptr_eq(&a1, &beta));
        assert!(!Arc::ptr_eq(&a1.storage, &beta.storage));
    }

    #[tokio::test]
    async fn a_space_runs_with_the_llm_and_config_of_its_org() {
        let tmp = tempfile::tempdir().unwrap();
        for (org, model, agent) in [
            ("acme", "acme-model", "ops"),
            ("globex", "globex-model", "dev"),
        ] {
            let path = assistant_core::org_config_path(tmp.path(), org).unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                path,
                format!(
                    "[llm]\nprovider = \"ollama\"\nmodel = \"{model}\"\n\n[agent]\nid = \"{agent}\"\n\n[memory]\nenabled = false\n"
                ),
            )
            .unwrap();
        }
        let runtimes = SpaceRuntimes::new(
            Arc::new(OrgPoolFactory::new(tmp.path())),
            Arc::new(OrgContexts::from_files(tmp.path())),
            None,
        );

        let acme = runtimes.get("acme", "main").await.unwrap();
        let globex = runtimes.get("globex", "main").await.unwrap();

        assert_eq!(acme.llm.model_name(), "acme-model");
        assert_eq!(acme.config.agent.id, "ops");
        assert_eq!(globex.llm.model_name(), "globex-model");
        assert_eq!(globex.config.agent.id, "dev");
    }

    #[tokio::test]
    async fn a_traversal_slug_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(runtimes(tmp.path()).get("acme", "../x").await.is_err());
    }

    struct Recording(Mutex<Vec<(String, String, Option<String>)>>);

    #[async_trait::async_trait]
    impl assistant_core::tool::TurnToolSource for Recording {
        async fn tools_for_turn(
            &self,
            scope: &assistant_core::tool::TurnScope<'_>,
        ) -> Vec<Arc<dyn ToolHandler>> {
            self.0.lock().unwrap().push((
                scope.org_slug.into(),
                scope.space_slug.into(),
                scope.user_id.map(|u| u.0.clone()),
            ));
            Vec::new()
        }
    }

    #[tokio::test]
    async fn every_space_asks_the_turn_tool_source_about_its_own_turns() {
        let tmp = tempfile::tempdir().unwrap();
        let source = Arc::new(Recording(Mutex::default()));
        let runtimes = {
            let mut config = OrgConfig::default();
            config.memory.enabled = false;
            Arc::new(
                SpaceRuntimes::new(
                    Arc::new(OrgPoolFactory::new(tmp.path())),
                    Arc::new(OrgContexts::fixed(
                        config,
                        Arc::new(ScriptedLlmProvider::new()),
                    )),
                    None,
                )
                .with_submit_timeout(10)
                .with_turn_tool_source(source.clone()),
            )
        };
        let mut auth = AuthContext::system();
        auth.user_id = assistant_core::identity::UserId::from("usr_u");

        for space in ["alpha", "beta"] {
            let runtime = runtimes.get("acme", space).await.unwrap();
            let worker = runtime.orchestrator.clone();
            tokio::spawn(async move { worker.run_worker("worker").await });
            tokio::time::timeout(
                Duration::from_secs(10),
                runtime.orchestrator.submit_turn(
                    &auth,
                    "hello",
                    Uuid::new_v4(),
                    Interface::Web,
                    None,
                    crate::orchestrator::SubmitOptions::default(),
                ),
            )
            .await
            .unwrap()
            .unwrap();
        }

        let mut seen = source.0.lock().unwrap().clone();
        seen.sort();
        assert_eq!(
            seen,
            vec![
                (
                    "acme".to_string(),
                    "alpha".to_string(),
                    Some("usr_u".to_string())
                ),
                (
                    "acme".to_string(),
                    "beta".to_string(),
                    Some("usr_u".to_string())
                ),
            ]
        );
    }

    #[tokio::test]
    async fn a_turn_submitted_in_one_space_persists_only_there() {
        let tmp = tempfile::tempdir().unwrap();
        let runtimes = runtimes(tmp.path());
        let alpha = runtimes.get("acme", "alpha").await.unwrap();
        let beta = runtimes.get("acme", "beta").await.unwrap();

        let worker = beta.orchestrator.clone();
        tokio::spawn(async move { worker.run_worker("beta-worker").await });

        let conversation = Uuid::new_v4();
        tokio::time::timeout(
            Duration::from_secs(10),
            beta.orchestrator.submit_turn(
                &AuthContext::system(),
                "hello",
                conversation,
                Interface::Web,
                None,
                crate::orchestrator::SubmitOptions::default(),
            ),
        )
        .await
        .unwrap()
        .unwrap();

        let history_in = |rt: &SpaceRuntime| {
            let store = SqliteConversationStore::for_agent(rt.storage.pool.clone(), "default");
            async move { store.load_history(conversation).await.unwrap() }
        };
        assert!(
            history_in(&beta).await.len() >= 2,
            "the turn lands in beta's database"
        );
        assert!(
            history_in(&alpha).await.is_empty(),
            "alpha's database is untouched"
        );
    }

    struct EchoTool;

    #[async_trait::async_trait]
    impl ToolHandler for EchoTool {
        fn name(&self) -> &str {
            "mcp-echo"
        }
        fn description(&self) -> &str {
            "echo"
        }
        fn params_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn run(
            &self,
            _params: std::collections::HashMap<String, serde_json::Value>,
            _ctx: &assistant_core::types::conversation::ExecutionContext,
        ) -> Result<assistant_core::tool::ToolOutput> {
            Ok(assistant_core::tool::ToolOutput::success("echo"))
        }
    }

    fn has_tool(rt: &SpaceRuntime, name: &str) -> bool {
        rt.executor.list_tools().iter().any(|t| t.name() == name)
    }

    #[tokio::test]
    async fn ambient_tools_reach_existing_and_later_spaces() {
        let tmp = tempfile::tempdir().unwrap();
        let runtimes = runtimes(tmp.path());
        let alpha = runtimes.get("acme", "alpha").await.unwrap();

        runtimes.register_ambient_tool(Arc::new(EchoTool));
        let beta = runtimes.get("acme", "beta").await.unwrap();
        assert!(has_tool(&alpha, "mcp-echo"));
        assert!(has_tool(&beta, "mcp-echo"));

        runtimes.unregister_tools_by_prefix("mcp-");
        let gamma = runtimes.get("acme", "gamma").await.unwrap();
        assert!(!has_tool(&alpha, "mcp-echo"));
        assert!(!has_tool(&gamma, "mcp-echo"));
    }

    fn mcp_skill(server: &str, name: &str) -> assistant_skills::SkillDef {
        assistant_skills::SkillDef {
            name: format!("{server}__{name}"),
            description: "remote".into(),
            license: None,
            compatibility: None,
            allowed_tools: Vec::new(),
            metadata: HashMap::new(),
            body: "body".into(),
            dir: std::path::PathBuf::from("/nonexistent"),
            source: SkillSource::Mcp {
                server: server.into(),
            },
        }
    }

    async fn has_skill(rt: &SpaceRuntime, name: &str) -> bool {
        rt.registry.get(name).await.is_some()
    }

    #[tokio::test]
    async fn ambient_skills_reach_existing_and_later_spaces() {
        let tmp = tempfile::tempdir().unwrap();
        let runtimes = runtimes(tmp.path());
        let alpha = runtimes.get("acme", "alpha").await.unwrap();

        runtimes
            .set_ambient_skills("remote", vec![mcp_skill("remote", "deploy")])
            .await;
        let beta = runtimes.get("acme", "beta").await.unwrap();
        assert!(has_skill(&alpha, "remote__deploy").await);
        assert!(has_skill(&beta, "remote__deploy").await);

        runtimes.set_ambient_skills("remote", Vec::new()).await;
        let gamma = runtimes.get("acme", "gamma").await.unwrap();
        assert!(!has_skill(&alpha, "remote__deploy").await);
        assert!(!has_skill(&gamma, "remote__deploy").await);
    }

    #[tokio::test]
    async fn a_shared_bus_is_namespaced_per_space() {
        use assistant_core::topic;
        let tmp = tempfile::tempdir().unwrap();
        let shared: Arc<dyn MessageBus> = Arc::new(assistant_storage::InMemoryMessageBus::new());
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        let runtimes = SpaceRuntimes::new(
            Arc::new(OrgPoolFactory::new(tmp.path())),
            Arc::new(OrgContexts::fixed(
                config,
                Arc::new(ScriptedLlmProvider::new()),
            )),
            None,
        )
        .with_shared_bus(shared);
        let alpha = runtimes.get("acme", "alpha").await.unwrap();
        let beta = runtimes.get("acme", "beta").await.unwrap();

        beta.bus
            .publish(assistant_core::PublishRequest::new(
                topic::TURN_REQUEST,
                serde_json::json!({}),
            ))
            .await
            .unwrap();

        assert!(
            alpha
                .bus
                .claim(topic::TURN_REQUEST, "alpha-worker")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            beta.bus
                .claim(topic::TURN_REQUEST, "beta-worker")
                .await
                .unwrap()
                .is_some()
        );
    }
}
