//! Common bootstrap — build the shared `StorageLayer` / `SkillRegistry` /
//! `ToolExecutor` / `Orchestrator` tuple used by every CLI subcommand.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

use assistant_core::types::agent::AssistantConfig;
use assistant_core::types::llm::{EmbeddingConfig, EmbeddingProviderKind};
use assistant_core::types::storage::BusKind;
use assistant_core::{
    EmbeddingProvider, LlmEmbedder, LlmProvider, MessageBus, WithEmbeddingOverride,
};
use assistant_llm_provider::{
    OllamaConfig, OllamaProvider, OpenAIProvider, OpenAIProviderConfig, VoyageConfig,
    VoyageEmbedder,
};
use assistant_runtime::{Orchestrator, orchestrator::ConfirmationCallback};
use assistant_skills::SkillSource;
use assistant_storage::{PersonaStore as _, StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

/// Build a dedicated [`EmbeddingProvider`] from an [`EmbeddingConfig`].
///
/// Falls back to provider-specific env vars for API keys.
fn build_embedding_provider(
    emb_cfg: &EmbeddingConfig,
    main_cfg: &assistant_core::types::llm::LlmConfig,
) -> Result<Arc<dyn EmbeddingProvider>> {
    match emb_cfg.provider {
        EmbeddingProviderKind::Ollama => {
            let ollama_cfg = OllamaConfig {
                model: "unused".to_string(),
                base_url: emb_cfg
                    .base_url
                    .clone()
                    .unwrap_or_else(|| main_cfg.base_url.clone()),
                timeout_secs: main_cfg.timeout_secs,
                embedding_model: emb_cfg
                    .model
                    .clone()
                    .unwrap_or_else(|| "nomic-embed-text".to_string()),
            };
            let provider = OllamaProvider::new(ollama_cfg)
                .context("Failed to create Ollama embedding provider")?;
            Ok(Arc::new(LlmEmbedder(Arc::new(provider))))
        }
        EmbeddingProviderKind::OpenAI => {
            let api_key = emb_cfg
                .api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "OpenAI embedding provider requires an API key. \
                         Set api_key in [llm.embeddings] or OPENAI_API_KEY env var."
                    )
                })?;
            let provider_cfg = OpenAIProviderConfig {
                model: "unused".to_string(),
                base_url: emb_cfg
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                timeout_secs: main_cfg.timeout_secs,
                max_tokens: 8192,
                embedding_model: emb_cfg
                    .model
                    .clone()
                    .unwrap_or_else(|| "text-embedding-3-small".to_string()),
                web_search: None,
            };
            let provider = OpenAIProvider::new(provider_cfg, &api_key)
                .context("Failed to create OpenAI embedding provider")?;
            Ok(Arc::new(LlmEmbedder(Arc::new(provider))))
        }
        EmbeddingProviderKind::Voyage => {
            let api_key = emb_cfg
                .api_key
                .clone()
                .or_else(|| std::env::var("VOYAGE_API_KEY").ok())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Voyage AI embedding provider requires an API key. \
                         Set api_key in [llm.embeddings] or VOYAGE_API_KEY env var."
                    )
                })?;
            let mut voyage_cfg = VoyageConfig::new(api_key);
            if let Some(ref url) = emb_cfg.base_url {
                voyage_cfg = voyage_cfg.with_base_url(url.clone());
            }
            if let Some(ref model) = emb_cfg.model {
                voyage_cfg = voyage_cfg.with_model(model.clone());
            }
            let embedder = VoyageEmbedder::new(voyage_cfg)
                .context("Failed to create Voyage AI embedding provider")?;
            Ok(Arc::new(embedder))
        }
    }
}

// ── Common bootstrap ──────────────────────────────────────────────────────────

pub struct Bootstrap {
    pub config: AssistantConfig,
    pub storage: Arc<StorageLayer>,
    pub registry: Arc<SkillRegistry>,
    pub executor: Arc<ToolExecutor>,
    pub orchestrator: Arc<Orchestrator>,
    pub user_skills_dir: PathBuf,
    pub llm: Arc<dyn LlmProvider>,
}

pub async fn bootstrap(
    home: &Path,
    confirmation_cb: Arc<dyn ConfirmationCallback>,
    storage: Arc<StorageLayer>,
    config: AssistantConfig,
) -> Result<Bootstrap> {
    let assistant_dir = home.join(".assistant");
    let user_skills_dir = assistant_dir.join("skills");

    // Build skill registry.
    let registry = SkillRegistry::new(storage.pool.clone())
        .await
        .context("Failed to create skill registry")?;

    let project_root = std::env::current_dir().ok();
    let dirs_to_scan = assistant_runtime::bootstrap::skill_dirs(&config, project_root.as_deref());
    let dirs_ref: Vec<(&Path, SkillSource)> = dirs_to_scan
        .iter()
        .map(|(p, s)| (p.as_path(), s.clone()))
        .collect();

    registry
        .load_embedded()
        .await
        .context("Failed to load embedded builtin skills")?;

    // Sync embedded builtin skills to ~/.assistant/skills/ so on-disk copies
    // stay in sync with the binary.  Stale or missing files are overwritten;
    // user skills with different names are never touched.
    if let Some(home) = dirs::home_dir() {
        let builtin_target = home.join(".assistant").join("skills");
        match registry.sync_builtins_to_disk(&builtin_target) {
            Ok(updated) if !updated.is_empty() => {
                tracing::info!(
                    "Synced {} built-in skill(s) to disk: {}",
                    updated.len(),
                    updated.join(", ")
                );
            }
            Err(e) => {
                tracing::warn!("Failed to sync built-in skills to disk: {e}");
            }
            _ => {}
        }
    }

    registry
        .load_from_dirs(&dirs_ref)
        .await
        .context("Failed to load skills from directories")?;

    let registry = Arc::new(registry);

    // Build LLM client — dispatch on configured provider.
    let llm: Arc<dyn LlmProvider> = assistant_llm_provider::create_provider(&config.llm)
        .context("Failed to create LLM provider")?;

    // Optionally wrap with a dedicated embedding provider.
    let llm: Arc<dyn LlmProvider> = if let Some(ref emb_cfg) = config.llm.embeddings {
        let embedder = build_embedding_provider(emb_cfg, &config.llm)
            .context("Failed to build dedicated embedding provider")?;
        info!(
            provider = ?emb_cfg.provider,
            model = emb_cfg.model.as_deref().unwrap_or("(default)"),
            "Using dedicated embedding provider"
        );
        Arc::new(WithEmbeddingOverride::new(llm, embedder))
    } else {
        llm
    };

    // Build tool executor.
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));

    // Build message bus.
    // When the `nats` feature is enabled and `[bus] kind = "nats"` is
    // configured, use NATS JetStream to avoid SQLite write-lock contention
    // between the bus and the rest of the storage layer.  Otherwise fall
    // back to the SQLite-backed bus.
    let bus: Arc<dyn MessageBus> = {
        #[cfg(feature = "nats")]
        {
            if config.bus.kind == BusKind::Nats {
                info!("Using NATS message bus");
                Arc::new(
                    assistant_bus_nats::NatsMessageBus::connect(&config.bus)
                        .await
                        .context("failed to connect to NATS")?,
                )
            } else {
                Arc::new(storage.message_bus())
            }
        }
        #[cfg(not(feature = "nats"))]
        {
            if config.bus.kind == BusKind::Nats {
                anyhow::bail!(
                    "[bus] kind = \"nats\" configured but this binary was built without the `nats` feature"
                );
            }
            Arc::new(storage.message_bus())
        }
    };

    // Build orchestrator, applying the per-persona turn timeout if set.
    let persona_timeout = storage
        .persona_store()
        .get(&config.agent.id)
        .await
        .ok()
        .flatten()
        .and_then(|p| p.turn_timeout_secs);
    let audio_store = Arc::new(assistant_transcription::AudioStore::new());
    let orchestrator = Arc::new({
        let mut o = Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        )
        .with_confirmation_callback(confirmation_cb)
        .with_audio_store(audio_store.clone());
        if let Some(secs) = persona_timeout {
            o = o.with_submit_timeout(secs);
        }
        o
    });

    // Wire up subagent support (breaks the init-time circular dep).
    executor.set_subagent_runner(orchestrator.clone());

    // Connect to configured external MCP servers and register their tools.
    if !config.mcp.servers.is_empty() {
        let mcp_manager =
            Arc::new(assistant_mcp_client::McpClientManager::start(&config.mcp.servers).await?);
        let mcp_tools = mcp_manager.tool_handlers().await;
        for handler in &mcp_tools {
            executor.register_ambient_tool(handler.clone());
        }
        info!(
            servers = mcp_manager.server_count().await,
            tools = mcp_tools.len(),
            "registered MCP client tools"
        );

        // Spawn background health-check loop for reconnection.
        let exec_register = executor.clone();
        let exec_unregister = executor.clone();
        mcp_manager.spawn_health_loop(
            Arc::new(move |h| exec_register.register_ambient_tool(h)),
            Arc::new(move |prefix| exec_unregister.unregister_tools_by_prefix(prefix)),
        );
    }

    // Keep a reference to the LLM for the memory indexer.
    let llm = orchestrator.llm.clone();

    Ok(Bootstrap {
        config,
        storage,
        registry,
        executor,
        orchestrator,
        user_skills_dir,
        llm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::llm::LlmConfig;

    fn base_llm_cfg() -> LlmConfig {
        let mut cfg = LlmConfig::default();
        cfg.base_url = "http://localhost:11434".to_string();
        cfg
    }

    #[test]
    fn ollama_embedding_provider_builds_without_keys() {
        let emb = EmbeddingConfig {
            provider: EmbeddingProviderKind::Ollama,
            model: Some("nomic-embed-text".to_string()),
            base_url: None,
            api_key: None,
        };
        let main = base_llm_cfg();
        build_embedding_provider(&emb, &main).expect("ollama doesn't need a key");
    }

    #[test]
    fn ollama_embedding_provider_uses_emb_base_url_when_set() {
        let emb = EmbeddingConfig {
            provider: EmbeddingProviderKind::Ollama,
            model: None,
            base_url: Some("http://elsewhere:11434".to_string()),
            api_key: None,
        };
        let main = base_llm_cfg();
        build_embedding_provider(&emb, &main).expect("ollama config accepted");
    }

    #[test]
    fn openai_embedding_provider_errors_without_key() {
        let emb = EmbeddingConfig {
            provider: EmbeddingProviderKind::OpenAI,
            model: None,
            base_url: None,
            api_key: None,
        };
        let main = base_llm_cfg();
        // Make sure the env var isn't bleeding in from the host shell.
        // SAFETY: test-only env mutation.
        let prev = std::env::var("OPENAI_API_KEY").ok();
        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
        }
        let err = build_embedding_provider(&emb, &main)
            .err()
            .expect("must error");
        assert!(err.to_string().contains("OpenAI"));
        if let Some(v) = prev {
            unsafe {
                std::env::set_var("OPENAI_API_KEY", v);
            }
        }
    }

    #[test]
    fn openai_embedding_provider_builds_when_key_set_in_config() {
        let emb = EmbeddingConfig {
            provider: EmbeddingProviderKind::OpenAI,
            model: Some("text-embedding-3-small".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            api_key: Some("sk-test".to_string()),
        };
        let main = base_llm_cfg();
        build_embedding_provider(&emb, &main).expect("explicit api_key is enough");
    }

    #[test]
    fn voyage_embedding_provider_errors_without_key() {
        let emb = EmbeddingConfig {
            provider: EmbeddingProviderKind::Voyage,
            model: None,
            base_url: None,
            api_key: None,
        };
        let main = base_llm_cfg();
        // SAFETY: test-only.
        let prev = std::env::var("VOYAGE_API_KEY").ok();
        unsafe {
            std::env::remove_var("VOYAGE_API_KEY");
        }
        let err = build_embedding_provider(&emb, &main)
            .err()
            .expect("must error");
        assert!(err.to_string().contains("Voyage") || err.to_string().contains("API key"));
        if let Some(v) = prev {
            unsafe {
                std::env::set_var("VOYAGE_API_KEY", v);
            }
        }
    }

    #[test]
    fn voyage_embedding_provider_builds_when_key_set_in_config() {
        let emb = EmbeddingConfig {
            provider: EmbeddingProviderKind::Voyage,
            model: Some("voyage-large-2".to_string()),
            base_url: None,
            api_key: Some("voyage-key".to_string()),
        };
        let main = base_llm_cfg();
        build_embedding_provider(&emb, &main).expect("explicit voyage key is enough");
    }
}
