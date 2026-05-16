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
