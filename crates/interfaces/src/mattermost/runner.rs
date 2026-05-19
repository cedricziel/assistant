//! Mattermost interface runner.
//!
//! Thin entry point — all dispatch logic lives in [`ChannelRunner`].

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};
use assistant_transcription::TranscriptionProvider;

use super::adapter::MattermostAdapter;
use super::client::MattermostClient;
use super::config::MattermostConfigExt;

pub use assistant_core::types::channels::MattermostConfig;

/// Mattermost interface runner. Connects via WebSocket and dispatches messages.
pub struct MattermostInterface {
    config: MattermostConfig,
    orchestrator: Arc<Orchestrator>,
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    transcription_language: Option<String>,
}

impl MattermostInterface {
    pub fn new(config: MattermostConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
            transcription: None,
            transcription_language: None,
        }
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
        let server_url = self.config.resolved_server_url().ok_or_else(|| {
            anyhow::anyhow!(
                "No Mattermost server URL configured. Set server_url in [mattermost] config \
                 or the MATTERMOST_SERVER_URL environment variable."
            )
        })?;
        let token = self.config.resolved_token().ok_or_else(|| {
            anyhow::anyhow!(
                "No Mattermost token configured. Set token in [mattermost] config \
                 or the MATTERMOST_TOKEN environment variable."
            )
        })?;

        let client = Arc::new(MattermostClient::new(&server_url, &token)?);
        let mut adapter = MattermostAdapter::new(
            client,
            self.config.allowed_channels.clone(),
            self.config.allowed_users.clone(),
        );
        if let Some(ref provider) = self.transcription {
            adapter =
                adapter.with_transcription(provider.clone(), self.transcription_language.clone());
        }

        ChannelRunner::new(Arc::new(adapter), self.orchestrator.clone())
            .run()
            .await
    }
}

#[async_trait::async_trait]
impl InterfaceRunner for MattermostInterface {
    async fn run(&self) -> Result<()> {
        MattermostInterface::run(self).await
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::types::channels::MattermostConfig;

    #[test]
    fn allowlist_channel_empty_accepts_all() {
        let cfg = MattermostConfig {
            allowed_channels: vec![],
            ..Default::default()
        };
        let channel = "town-square".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&channel);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_channel_non_empty_blocks_unknown() {
        let cfg = MattermostConfig {
            allowed_channels: vec!["bot-test".to_string()],
            ..Default::default()
        };
        let unknown = "town-square".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_user_non_empty_passes_known() {
        let cfg = MattermostConfig {
            allowed_users: vec!["alice".to_string()],
            ..Default::default()
        };
        let known = "alice".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&known);
        assert!(!blocked);
    }

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
    async fn new_starts_without_transcription() {
        let orch = make_orchestrator().await;
        let iface = MattermostInterface::new(MattermostConfig::default(), orch);
        assert!(iface.transcription.is_none());
        assert!(iface.transcription_language.is_none());
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
            async fn transcribe(&self, _req: TranscriptionRequest) -> Result<TranscriptionResult> {
                Ok(TranscriptionResult {
                    text: "x".into(),
                    language: None,
                    duration_secs: None,
                })
            }
        }
        let orch = make_orchestrator().await;
        let iface = MattermostInterface::new(MattermostConfig::default(), orch)
            .with_transcription(Arc::new(DummyProvider), Some("en".into()));
        assert!(iface.transcription.is_some());
        assert_eq!(iface.transcription_language.as_deref(), Some("en"));
    }

    #[tokio::test]
    async fn run_errors_when_server_url_missing() {
        let orch = make_orchestrator().await;
        let iface = MattermostInterface::new(
            MattermostConfig {
                server_url: None,
                token: Some("tok".into()),
                ..Default::default()
            },
            orch,
        );
        let err = iface.run().await.unwrap_err();
        assert!(err.to_string().contains("Mattermost server URL"));
    }

    #[tokio::test]
    async fn run_errors_when_token_missing() {
        let orch = make_orchestrator().await;
        let iface = MattermostInterface::new(
            MattermostConfig {
                server_url: Some("https://mm.example.com".into()),
                token: None,
                ..Default::default()
            },
            orch,
        );
        let err = iface.run().await.unwrap_err();
        assert!(err.to_string().contains("Mattermost token"));
    }
}
