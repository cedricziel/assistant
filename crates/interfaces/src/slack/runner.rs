//! Slack interface runner.
//!
//! Thin entry point — all dispatch logic lives in [`ChannelRunner`].

use std::sync::Arc;

use anyhow::Result;
use assistant_core::types::channels::SlackConfig;
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};
use assistant_storage::StorageLayer;
use assistant_transcription::TranscriptionProvider;
use tracing::warn;

use super::adapter::SlackAdapter;
use super::config::SlackConfigExt;
use super::skills::{
    SlackDeleteMessageSkill, SlackGetHistorySkill, SlackListChannelsSkill, SlackLookupUserSkill,
    SlackPostSkill, SlackReactSkill, SlackSendDmSkill, SlackUpdateMessageSkill,
};

/// Slack interface runner.  Connects via Socket Mode and dispatches messages.
pub struct SlackInterface {
    config: SlackConfig,
    orchestrator: Arc<Orchestrator>,
    storage: Arc<StorageLayer>,
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    transcription_language: Option<String>,
}

impl SlackInterface {
    pub fn new(
        config: SlackConfig,
        orchestrator: Arc<Orchestrator>,
        storage: Arc<StorageLayer>,
    ) -> Self {
        Self {
            config,
            orchestrator,
            storage,
            transcription: None,
            transcription_language: None,
        }
    }

    /// Enable automatic audio transcription for voice messages.
    pub fn with_transcription(
        mut self,
        provider: Arc<dyn TranscriptionProvider>,
        language: Option<String>,
    ) -> Self {
        self.transcription = Some(provider);
        self.transcription_language = language;
        self
    }

    /// Return ambient tools contributed by this interface.
    pub fn ambient_tools(&self) -> Vec<std::sync::Arc<dyn assistant_core::ToolHandler>> {
        let Some(bot_token) = self.config.resolved_bot_token() else {
            return vec![];
        };
        let Some(app_token) = self.config.resolved_app_token() else {
            return vec![];
        };
        let client = match super::client::SlackApiClient::new(bot_token, app_token) {
            Ok(c) => std::sync::Arc::new(c),
            Err(e) => {
                warn!(error = %e, "slack: failed to build ambient API client");
                return vec![];
            }
        };
        vec![
            std::sync::Arc::new(SlackPostSkill {
                client: client.clone(),
            }) as std::sync::Arc<dyn assistant_core::ToolHandler>,
            std::sync::Arc::new(SlackSendDmSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackListChannelsSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackGetHistorySkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackReactSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackUpdateMessageSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackDeleteMessageSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackLookupUserSkill { client }),
        ]
    }

    /// Run the Slack interface: connect, receive messages, dispatch to orchestrator.
    pub async fn run(&self) -> Result<()> {
        let mut adapter =
            SlackAdapter::new(self.config.clone())?.with_storage(self.storage.clone());
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
impl InterfaceRunner for SlackInterface {
    async fn run(&self) -> Result<()> {
        SlackInterface::run(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::types::channels::SlackConfig;

    fn config_with_tokens(bot: Option<&str>, app: Option<&str>) -> SlackConfig {
        SlackConfig {
            bot_token: bot.map(String::from),
            app_token: app.map(String::from),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn ambient_tools_returns_empty_when_bot_token_missing() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let storage = Arc::new(storage);
        // We don't actually need a real Orchestrator for ambient_tools; build one
        // off the in-memory storage so the struct fields are populated.
        let orch = make_orchestrator(storage.clone()).await;
        let iface = SlackInterface::new(config_with_tokens(None, Some("xapp-1")), orch, storage);
        assert!(iface.ambient_tools().is_empty());
    }

    #[tokio::test]
    async fn ambient_tools_returns_empty_when_app_token_missing() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let storage = Arc::new(storage);
        let orch = make_orchestrator(storage.clone()).await;
        let iface = SlackInterface::new(config_with_tokens(Some("xoxb-1"), None), orch, storage);
        assert!(iface.ambient_tools().is_empty());
    }

    #[tokio::test]
    async fn ambient_tools_returns_eight_tools_when_both_tokens_present() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let storage = Arc::new(storage);
        let orch = make_orchestrator(storage.clone()).await;
        let iface = SlackInterface::new(
            config_with_tokens(Some("xoxb-1"), Some("xapp-1")),
            orch,
            storage,
        );
        let tools = iface.ambient_tools();
        assert_eq!(tools.len(), 8, "should expose all 8 ambient skills");
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.iter().any(|n| n.contains("post")));
        assert!(names.iter().any(|n| n.contains("react")));
    }

    #[tokio::test]
    async fn with_transcription_sets_provider() {
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

        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let storage = Arc::new(storage);
        let orch = make_orchestrator(storage.clone()).await;
        let iface = SlackInterface::new(SlackConfig::default(), orch, storage);
        assert!(iface.transcription.is_none());
        let iface = iface.with_transcription(Arc::new(DummyProvider), Some("en".into()));
        assert!(iface.transcription.is_some());
        assert_eq!(iface.transcription_language.as_deref(), Some("en"));
    }

    async fn make_orchestrator(storage: Arc<StorageLayer>) -> Arc<Orchestrator> {
        use assistant_core::types::agent::AssistantConfig;
        use assistant_core::{LlmProvider, MessageBus};
        use assistant_llm_provider::scripted::ScriptedLlmProvider;
        use assistant_storage::registry::SkillRegistry;
        use assistant_tool_executor::ToolExecutor;
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
            exec.clone(),
            registry.clone(),
            bus,
            &cfg,
        ))
    }
}
