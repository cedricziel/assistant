//! Matrix interface runner.
//!
//! Thin entry point — all dispatch logic lives in [`ChannelRunner`].

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};
use assistant_transcription::TranscriptionProvider;

use super::adapter::MatrixAdapter;
use super::client::MatrixClient;
use super::config::MatrixConfigExt;

pub use assistant_core::types::channels::MatrixConfig;

/// Matrix interface runner.
pub struct MatrixInterface {
    config: MatrixConfig,
    orchestrator: Arc<Orchestrator>,
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    transcription_language: Option<String>,
}

impl MatrixInterface {
    pub fn new(config: MatrixConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
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

    pub async fn run(&self) -> Result<()> {
        let homeserver_url = self.config.resolved_homeserver_url().ok_or_else(|| {
            anyhow::anyhow!(
                "No Matrix homeserver URL configured. Set homeserver_url in [matrix] config \
                 or MATRIX_HOMESERVER_URL env var."
            )
        })?;

        // Authenticate.
        let client = Arc::new(if let Some(token) = self.config.resolved_access_token() {
            let username = self.config.resolved_username().ok_or_else(|| {
                anyhow::anyhow!(
                    "access_token is set but username is missing. \
                     Set username in [matrix] config or MATRIX_USERNAME env var."
                )
            })?;
            tracing::info!("Matrix: using access token auth");
            MatrixClient::new_with_token(&homeserver_url, token, username)?
        } else {
            let username = self.config.resolved_username().ok_or_else(|| {
                anyhow::anyhow!(
                    "No Matrix credentials. Set access_token or username+password in [matrix] config."
                )
            })?;
            let password = self
                .config
                .resolved_password()
                .ok_or_else(|| anyhow::anyhow!("username is set but password is missing."))?;
            tracing::info!(username = %username, "Matrix: logging in with password");
            MatrixClient::login(&homeserver_url, &username, &password).await?
        });

        tracing::info!(user_id = %client.user_id, "Matrix: authenticated");

        let mut adapter = MatrixAdapter::new(
            client,
            self.config.allowed_rooms.clone(),
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
impl InterfaceRunner for MatrixInterface {
    async fn run(&self) -> Result<()> {
        MatrixInterface::run(self).await
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::types::channels::MatrixConfig;

    #[test]
    fn allowlist_room_empty_accepts_all() {
        let cfg = MatrixConfig {
            allowed_rooms: vec![],
            ..Default::default()
        };
        let room_id = "!abc:example.com".to_string();
        let blocked = !cfg.allowed_rooms.is_empty() && !cfg.allowed_rooms.contains(&room_id);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_room_non_empty_blocks_unknown() {
        let cfg = MatrixConfig {
            allowed_rooms: vec!["!known:example.com".to_string()],
            ..Default::default()
        };
        let blocked = !cfg.allowed_rooms.is_empty()
            && !cfg
                .allowed_rooms
                .contains(&"!other:example.com".to_string());
        assert!(blocked);
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
        let iface = MatrixInterface::new(MatrixConfig::default(), orch);
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
        let iface = MatrixInterface::new(MatrixConfig::default(), orch)
            .with_transcription(Arc::new(DummyProvider), Some("en".into()));
        assert!(iface.transcription.is_some());
        assert_eq!(iface.transcription_language.as_deref(), Some("en"));
    }

    #[tokio::test]
    async fn run_errors_when_homeserver_missing() {
        let orch = make_orchestrator().await;
        let iface = MatrixInterface::new(
            MatrixConfig {
                homeserver_url: None,
                ..Default::default()
            },
            orch,
        );
        let err = iface.run().await.unwrap_err();
        assert!(err.to_string().contains("homeserver URL"));
    }

    #[tokio::test]
    async fn run_errors_when_only_access_token_but_no_username() {
        let orch = make_orchestrator().await;
        let iface = MatrixInterface::new(
            MatrixConfig {
                homeserver_url: Some("https://matrix.example.com".into()),
                access_token: Some("t".into()),
                username: None,
                ..Default::default()
            },
            orch,
        );
        let err = iface.run().await.unwrap_err();
        assert!(err.to_string().contains("username"));
    }

    #[tokio::test]
    async fn run_errors_when_no_credentials() {
        let orch = make_orchestrator().await;
        let iface = MatrixInterface::new(
            MatrixConfig {
                homeserver_url: Some("https://matrix.example.com".into()),
                access_token: None,
                username: None,
                password: None,
                ..Default::default()
            },
            orch,
        );
        let err = iface.run().await.unwrap_err();
        assert!(err.to_string().contains("credentials"));
    }
}
