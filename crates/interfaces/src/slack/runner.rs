//! Slack interface runner.
//!
//! Thin entry point — all dispatch logic lives in [`ChannelRunner`].

use std::sync::Arc;

use anyhow::Result;
use assistant_core::SlackConfig;
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
