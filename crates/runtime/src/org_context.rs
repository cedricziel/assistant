//! The settings and LLM client of one organization.
//!
//! Each org carries its own `org.toml`, so a space gets its LLM provider and
//! feature settings from the org it belongs to. [`OrgContexts`] loads them on
//! first use and caches them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result};

use assistant_core::types::agent::OrgConfig;
use assistant_core::types::llm::{EmbeddingConfig, EmbeddingProviderKind, LlmConfig};
use assistant_core::{
    EmbeddingProvider, LlmEmbedder, LlmProvider, Secret, WithEmbeddingOverride, load_org_config,
};
use assistant_llm_provider::{
    OllamaConfig, OllamaProvider, OpenAIProvider, OpenAIProviderConfig, VoyageConfig,
    VoyageEmbedder,
};

/// Adjusts a freshly loaded [`OrgConfig`], e.g. to apply command-line overrides.
pub type OrgConfigAdjust = Arc<dyn Fn(&mut OrgConfig) -> Result<()> + Send + Sync>;

/// An org's settings together with the LLM client built from them.
#[derive(Clone)]
pub struct OrgContext {
    pub config: Arc<OrgConfig>,
    pub llm: Arc<dyn LlmProvider>,
}

enum Source {
    /// Read `orgs/{slug}/org.toml` under `base_dir`.
    Files {
        base_dir: PathBuf,
        adjust: Option<OrgConfigAdjust>,
    },
    /// Serve one context to every org.
    Fixed(OrgContext),
}

/// Resolves an org slug to its [`OrgContext`], built once per org.
pub struct OrgContexts {
    source: Source,
    cache: Mutex<HashMap<String, OrgContext>>,
}

impl OrgContexts {
    /// Read each org's `org.toml` from the install root `base_dir`.
    pub fn from_files(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            source: Source::Files {
                base_dir: base_dir.into(),
                adjust: None,
            },
            cache: Mutex::default(),
        }
    }

    /// Run `adjust` on every org config right after it is loaded.
    pub fn with_adjust(mut self, adjust: OrgConfigAdjust) -> Self {
        if let Source::Files { adjust: slot, .. } = &mut self.source {
            *slot = Some(adjust);
        }
        self
    }

    /// Give every org `config` and `llm`, without touching the filesystem.
    pub fn fixed(config: OrgConfig, llm: Arc<dyn LlmProvider>) -> Self {
        Self {
            source: Source::Fixed(OrgContext {
                config: Arc::new(config),
                llm,
            }),
            cache: Mutex::default(),
        }
    }

    /// The context of `org_slug`, loaded on first use.
    pub fn get(&self, org_slug: &str) -> Result<OrgContext> {
        let (base_dir, adjust) = match &self.source {
            Source::Fixed(ctx) => return Ok(ctx.clone()),
            Source::Files { base_dir, adjust } => (base_dir, adjust),
        };
        if let Some(ctx) = self
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(org_slug)
        {
            return Ok(ctx.clone());
        }
        let mut config = load_org_config(base_dir, org_slug)?;
        if let Some(adjust) = adjust {
            adjust(&mut config).with_context(|| format!("adjusting config of org {org_slug}"))?;
        }
        let llm = build_llm(&config.llm)
            .with_context(|| format!("building the LLM client of org {org_slug}"))?;
        let ctx = OrgContext {
            config: Arc::new(config),
            llm,
        };
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(org_slug.to_owned(), ctx.clone());
        Ok(ctx)
    }

    /// Forget an org's cached context so the next [`get`](Self::get) rereads its file.
    pub fn evict(&self, org_slug: &str) {
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(org_slug);
    }
}

/// The configured chat provider, wrapped with the dedicated embedding provider
/// when `[llm.embeddings]` is set.
pub fn build_llm(cfg: &LlmConfig) -> Result<Arc<dyn LlmProvider>> {
    let llm =
        assistant_llm_provider::create_provider(cfg).context("Failed to create LLM provider")?;
    let Some(emb_cfg) = &cfg.embeddings else {
        return Ok(llm);
    };
    let embedder = build_embedding_provider(emb_cfg, cfg)
        .context("Failed to build dedicated embedding provider")?;
    tracing::info!(
        provider = ?emb_cfg.provider,
        model = emb_cfg.model.as_deref().unwrap_or("(default)"),
        "Using dedicated embedding provider"
    );
    Ok(Arc::new(WithEmbeddingOverride::new(llm, embedder)))
}

/// Build a dedicated [`EmbeddingProvider`] from an [`EmbeddingConfig`].
///
/// Falls back to provider-specific env vars for API keys.
fn build_embedding_provider(
    emb_cfg: &EmbeddingConfig,
    main_cfg: &LlmConfig,
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

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::org_config_path;
    use assistant_core::types::llm::LlmConfig;

    fn write_org(base: &std::path::Path, slug: &str, content: &str) {
        let path = org_config_path(base, slug).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn each_org_gets_the_llm_of_its_own_org_toml() {
        let tmp = tempfile::tempdir().unwrap();
        write_org(
            tmp.path(),
            "acme",
            "[llm]\nprovider = \"ollama\"\nmodel = \"acme-model\"\n",
        );
        write_org(
            tmp.path(),
            "globex",
            "[llm]\nprovider = \"ollama\"\nmodel = \"globex-model\"\n",
        );
        let orgs = OrgContexts::from_files(tmp.path());

        let acme = orgs.get("acme").unwrap();
        let globex = orgs.get("globex").unwrap();
        assert_eq!(acme.llm.model_name(), "acme-model");
        assert_eq!(globex.llm.model_name(), "globex-model");
        assert_eq!(acme.config.llm.model, "acme-model");
    }

    #[test]
    fn an_org_is_loaded_once_until_evicted() {
        let tmp = tempfile::tempdir().unwrap();
        write_org(tmp.path(), "acme", "[llm]\nmodel = \"first\"\n");
        let orgs = OrgContexts::from_files(tmp.path());
        let first = orgs.get("acme").unwrap();
        assert!(Arc::ptr_eq(
            &first.config,
            &orgs.get("acme").unwrap().config
        ));

        write_org(tmp.path(), "acme", "[llm]\nmodel = \"second\"\n");
        assert_eq!(orgs.get("acme").unwrap().llm.model_name(), "first");
        orgs.evict("acme");
        assert_eq!(orgs.get("acme").unwrap().llm.model_name(), "second");
    }

    #[test]
    fn adjust_runs_on_every_loaded_org() {
        let tmp = tempfile::tempdir().unwrap();
        write_org(tmp.path(), "acme", "[llm]\nmodel = \"from-file\"\n");
        let orgs = OrgContexts::from_files(tmp.path()).with_adjust(Arc::new(|config| {
            config.llm.model = "from-flag".to_string();
            Ok(())
        }));
        assert_eq!(orgs.get("acme").unwrap().llm.model_name(), "from-flag");
        assert_eq!(orgs.get("other").unwrap().llm.model_name(), "from-flag");
    }

    #[test]
    fn a_malformed_org_toml_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        write_org(tmp.path(), "acme", "{{{{");
        assert!(OrgContexts::from_files(tmp.path()).get("acme").is_err());
    }

    #[test]
    fn a_traversal_slug_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(OrgContexts::from_files(tmp.path()).get("../x").is_err());
    }

    #[test]
    fn fixed_serves_the_same_context_to_every_org() {
        let llm: Arc<dyn LlmProvider> =
            Arc::new(assistant_llm_provider::scripted::ScriptedLlmProvider::new());
        let orgs = OrgContexts::fixed(OrgConfig::default(), llm);
        assert!(Arc::ptr_eq(
            &orgs.get("a").unwrap().config,
            &orgs.get("b").unwrap().config
        ));
    }

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
