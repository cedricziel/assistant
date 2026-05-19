//! Signal interface runner.
//!
//! [`SignalInterface`] delegates the receive/dispatch loop to
//! [`ChannelRunner`], which drives all messenger interfaces in this workspace.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::types::channels::SignalConfig;
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};
use assistant_transcription::TranscriptionProvider;
use async_trait::async_trait;

use super::adapter::SignalAdapter;

/// The Signal interface handle.
pub struct SignalInterface {
    config: SignalConfig,
    orchestrator: Arc<Orchestrator>,
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    transcription_language: Option<String>,
}

impl SignalInterface {
    /// Create a new [`SignalInterface`].
    pub fn new(config: SignalConfig, orchestrator: Arc<Orchestrator>) -> Self {
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

    /// Start the Signal listener loop via [`ChannelRunner`].
    pub async fn run(&self) -> Result<()> {
        let mut adapter = SignalAdapter::new(self.config.clone())?;
        if let Some(ref provider) = self.transcription {
            adapter =
                adapter.with_transcription(provider.clone(), self.transcription_language.clone());
        }
        ChannelRunner::new(Arc::new(adapter), self.orchestrator.clone())
            .run()
            .await
    }
}

#[async_trait]
impl InterfaceRunner for SignalInterface {
    async fn run(&self) -> Result<()> {
        self.run().await
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::types::channels::SignalConfig;

    #[test]
    fn allowlist_logic_empty_accepts_all() {
        let cfg = SignalConfig {
            allowed_senders: vec![],
            ..Default::default()
        };
        let sender = "some-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&sender);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_logic_non_empty_blocks_unknown() {
        let cfg = SignalConfig {
            allowed_senders: vec!["allowed-uuid".to_string()],
            ..Default::default()
        };
        let unknown = "unknown-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_logic_non_empty_passes_known() {
        let cfg = SignalConfig {
            allowed_senders: vec!["allowed-uuid".to_string()],
            ..Default::default()
        };
        let known = "allowed-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&known);
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
        let iface = SignalInterface::new(SignalConfig::default(), orch);
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
        #[async_trait]
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
        let iface = SignalInterface::new(SignalConfig::default(), orch)
            .with_transcription(Arc::new(DummyProvider), Some("en".into()));
        assert!(iface.transcription.is_some());
        assert_eq!(iface.transcription_language.as_deref(), Some("en"));
    }
}
