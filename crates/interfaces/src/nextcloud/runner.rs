//! Nextcloud Talk webhook-based bot interface.
//!
//! Unlike Slack and Mattermost (which use WebSocket connections), the
//! Nextcloud Talk Bot API is webhook-based — the Nextcloud server POSTs
//! Activity Streams 2.0 events to the bot's HTTP endpoint.
//!
//! [`NextcloudAdapter::start()`] spawns the axum HTTP server internally and
//! exposes incoming messages as a [`Stream`].  [`ChannelRunner`] drives the
//! rest of the dispatch loop.

use std::sync::Arc;

use anyhow::{Context, Result};
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};
use assistant_transcription::TranscriptionProvider;
use tracing::info;

use super::adapter::NextcloudAdapter;
use super::config::NextcloudConfigExt;

pub use assistant_core::types::channels::NextcloudConfig;

/// The Nextcloud Talk bot interface.
///
/// Runs an HTTP server that receives webhook events from the Nextcloud Talk
/// server and processes them through the orchestrator via [`ChannelRunner`].
pub struct NextcloudInterface {
    config: NextcloudConfig,
    orchestrator: Arc<Orchestrator>,
    /// Optional external shutdown signal for background / daemon mode.
    shutdown: Option<tokio_util::sync::CancellationToken>,
    /// Optional audio transcription provider for voice messages.
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    /// BCP-47 language hint passed to the transcription provider.
    transcription_language: Option<String>,
}

impl NextcloudInterface {
    pub fn new(config: NextcloudConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
            shutdown: None,
            transcription: None,
            transcription_language: None,
        }
    }

    /// Attach an external shutdown token.  When cancelled, the HTTP server
    /// and dispatch loop shut down gracefully without installing process-wide
    /// signal handlers.  Used in background mode to avoid conflicting with the
    /// CLI REPL's Ctrl-C handling.
    pub fn with_shutdown(mut self, token: tokio_util::sync::CancellationToken) -> Self {
        self.shutdown = Some(token);
        self
    }

    /// Attach a transcription provider for inbound voice messages.
    pub fn with_transcription(
        mut self,
        provider: Arc<dyn TranscriptionProvider>,
        language: Option<String>,
    ) -> Self {
        self.transcription = Some(provider);
        self.transcription_language = language;
        self
    }

    pub async fn run(&self) -> Result<()> {
        let server_url = self
            .config
            .resolved_server_url()
            .context("Nextcloud server_url is required. Set it in [nextcloud] config or NEXTCLOUD_SERVER_URL env var")?;

        info!(
            server_url = %server_url,
            listen_addr = %self.config.listen_addr,
            "Starting Nextcloud Talk bot"
        );

        let mut adapter = NextcloudAdapter::new(self.config.clone())
            .context("Failed to create NextcloudAdapter")?;
        if let Some(ref provider) = self.transcription {
            adapter =
                adapter.with_transcription(provider.clone(), self.transcription_language.clone());
        }
        let adapter = Arc::new(adapter);

        let mut runner = ChannelRunner::new(adapter, self.orchestrator.clone());
        if let Some(token) = &self.shutdown {
            runner = runner.with_shutdown(token.clone());
        }
        runner.run().await
    }
}

#[async_trait::async_trait]
impl InterfaceRunner for NextcloudInterface {
    async fn run(&self) -> Result<()> {
        self.run().await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use super::*;
    use assistant_storage::StorageLayer;

    async fn make_orchestrator() -> Arc<Orchestrator> {
        use assistant_core::types::agent::AssistantConfig;
        use assistant_core::{LlmProvider, MessageBus};
        use assistant_llm_provider::scripted::ScriptedLlmProvider;
        use assistant_storage::registry::SkillRegistry;
        use assistant_tool_executor::ToolExecutor;
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let cfg = AssistantConfig::default();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(ScriptedLlmProvider::new());
        let exec = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(cfg.clone()),
        ));
        let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            exec,
            registry,
            bus,
            &cfg,
        ))
    }

    #[tokio::test]
    async fn new_starts_without_shutdown_or_transcription() {
        let orch = make_orchestrator().await;
        let iface = NextcloudInterface::new(NextcloudConfig::default(), orch);
        assert!(iface.shutdown.is_none());
        assert!(iface.transcription.is_none());
        assert!(iface.transcription_language.is_none());
    }

    #[tokio::test]
    async fn with_shutdown_attaches_token() {
        let orch = make_orchestrator().await;
        let token = tokio_util::sync::CancellationToken::new();
        let iface =
            NextcloudInterface::new(NextcloudConfig::default(), orch).with_shutdown(token.clone());
        assert!(iface.shutdown.is_some());
    }

    #[tokio::test]
    async fn with_transcription_sets_provider_and_language() {
        use assistant_transcription::{
            TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
        };
        #[derive(Debug)]
        struct DummyProvider;
        #[async_trait::async_trait]
        impl TranscriptionProvider for DummyProvider {
            fn name(&self) -> &str {
                "dummy"
            }
            async fn transcribe(
                &self,
                _req: TranscriptionRequest,
            ) -> anyhow::Result<TranscriptionResult> {
                Ok(TranscriptionResult {
                    text: "x".into(),
                    language: None,
                    duration_secs: None,
                })
            }
        }
        let orch = make_orchestrator().await;
        let iface = NextcloudInterface::new(NextcloudConfig::default(), orch)
            .with_transcription(Arc::new(DummyProvider), Some("en".into()));
        assert!(iface.transcription.is_some());
        assert_eq!(iface.transcription_language.as_deref(), Some("en"));
    }

    #[tokio::test]
    async fn run_errors_when_server_url_missing() {
        let orch = make_orchestrator().await;
        // SAFETY: tests run with default env; clear any inherited value.
        unsafe {
            std::env::remove_var("NEXTCLOUD_SERVER_URL");
        }
        let iface = NextcloudInterface::new(
            NextcloudConfig {
                server_url: None,
                ..Default::default()
            },
            orch,
        );
        let err = iface.run().await.unwrap_err();
        assert!(err.to_string().contains("server_url"));
    }

    #[test]
    fn conversation_token_mapping_stable() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let conversations: Mutex<HashMap<String, Uuid>> = Mutex::new(HashMap::new());

            let token = "abc123";
            let id1 = {
                let mut map = conversations.lock().await;
                *map.entry(token.to_string()).or_insert_with(Uuid::new_v4)
            };
            let id2 = {
                let mut map = conversations.lock().await;
                *map.entry(token.to_string()).or_insert_with(Uuid::new_v4)
            };
            assert_eq!(id1, id2, "same conversation token must map to same UUID");

            let id3 = {
                let mut map = conversations.lock().await;
                *map.entry("other_token".to_string())
                    .or_insert_with(Uuid::new_v4)
            };
            assert_ne!(id1, id3, "different tokens must produce different UUIDs");
        });
    }
}
