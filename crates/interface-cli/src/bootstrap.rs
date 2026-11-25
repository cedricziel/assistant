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
    EmbeddingProvider, LlmEmbedder, LlmProvider, MessageBus, Secret, WithEmbeddingOverride,
};
use assistant_llm_provider::{
    OllamaConfig, OllamaProvider, OpenAIProvider, OpenAIProviderConfig, VoyageConfig,
    VoyageEmbedder,
};
use assistant_runtime::{Orchestrator, SpaceRuntimes, orchestrator::ConfirmationCallback};
use assistant_storage::migration::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG};
use assistant_storage::{OrgPoolFactory, StorageLayer, registry::SkillRegistry};
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
                .or_else(|| std::env::var("OPENAI_API_KEY").ok().map(Secret::from))
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
            let provider = OpenAIProvider::new(provider_cfg, api_key.expose())
                .context("Failed to create OpenAI embedding provider")?;
            Ok(Arc::new(LlmEmbedder(Arc::new(provider))))
        }
        EmbeddingProviderKind::Voyage => {
            let api_key = emb_cfg
                .api_key
                .clone()
                .or_else(|| std::env::var("VOYAGE_API_KEY").ok().map(Secret::from))
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

/// This process's runtimes: one per space, built on demand, plus the default
/// space's stack for the REPL, one-shot turns and the MCP server.
pub struct Bootstrap {
    pub config: AssistantConfig,
    pub runtimes: Arc<SpaceRuntimes>,
    pub storage: Arc<StorageLayer>,
    pub registry: Arc<SkillRegistry>,
    pub executor: Arc<ToolExecutor>,
    pub orchestrator: Arc<Orchestrator>,
    pub user_skills_dir: PathBuf,
}

/// The broker shared by every space, when `[bus] kind = "nats"`; otherwise each
/// space uses the SQLite bus in its own database.
async fn shared_nats_bus(config: &AssistantConfig) -> Result<Option<Arc<dyn MessageBus>>> {
    if config.bus.kind != BusKind::Nats {
        return Ok(None);
    }
    #[cfg(feature = "nats")]
    {
        info!("Using NATS message bus");
        let bus = assistant_bus_nats::NatsMessageBus::connect(&config.bus)
            .await
            .context("failed to connect to NATS")?;
        Ok(Some(Arc::new(bus)))
    }
    #[cfg(not(feature = "nats"))]
    {
        anyhow::bail!(
            "[bus] kind = \"nats\" configured but this binary was built without the `nats` feature"
        )
    }
}

pub async fn bootstrap(
    home: &Path,
    confirmation_cb: Arc<dyn ConfirmationCallback>,
    pools: Arc<OrgPoolFactory>,
    config: AssistantConfig,
) -> Result<Bootstrap> {
    let user_skills_dir = home.join(".assistant").join("skills");

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

    // With the `nats` feature and `[bus] kind = "nats"`, turns travel over NATS
    // JetStream (namespaced per space) to avoid SQLite write-lock contention
    // between the bus and the rest of the storage layer.
    let mut runtimes = SpaceRuntimes::new(
        pools.clone(),
        Arc::new(config.clone()),
        llm.clone(),
        confirmation_cb,
    )
    .with_audio_store(Arc::new(assistant_transcription::AudioStore::new()));
    if let Some(bus) = shared_nats_bus(&config).await? {
        runtimes = runtimes.with_shared_bus(bus);
    }
    let runtimes = Arc::new(runtimes);

    // Connect to configured external MCP servers and register their tools in
    // every space.
    if !config.mcp.servers.is_empty() {
        let mcp_manager =
            Arc::new(assistant_mcp_client::McpClientManager::start(&config.mcp.servers).await?);
        let mcp_tools = mcp_manager.tool_handlers().await;
        for handler in &mcp_tools {
            runtimes.register_ambient_tool(handler.clone());
        }
        info!(
            servers = mcp_manager.server_count().await,
            tools = mcp_tools.len(),
            "registered MCP client tools"
        );

        // Spawn background health-check loop for reconnection.
        let register = runtimes.clone();
        let unregister = runtimes.clone();
        mcp_manager.spawn_health_loop(
            Arc::new(move |h| register.register_ambient_tool(h)),
            Arc::new(move |prefix| unregister.unregister_tools_by_prefix(prefix)),
        );
    }

    let default_space = runtimes
        .get(DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG)
        .await
        .context("Failed to open the default space runtime")?;

    Ok(Bootstrap {
        config,
        runtimes,
        storage: default_space.storage.clone(),
        registry: default_space.registry.clone(),
        executor: default_space.executor.clone(),
        orchestrator: default_space.orchestrator.clone(),
        user_skills_dir,
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
            api_key: Some("sk-test".into()),
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
            api_key: Some("voyage-key".into()),
        };
        let main = base_llm_cfg();
        build_embedding_provider(&emb, &main).expect("explicit voyage key is enough");
    }
}
