//! Mattermost interface runner.
//!
//! Drives the [`MattermostAdapter`] stream → orchestrator dispatch loop.
//! Each inbound [`ChannelMessage`] is dispatched to the orchestrator with
//! per-conversation serialisation (one turn at a time per thread).

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ChannelAdapter, ChannelContent, ChannelMessage, Interface};
use assistant_runtime::{InterfaceRunner, Orchestrator};
use futures::StreamExt;
use lru::LruCache;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::adapter::MattermostAdapter;
use crate::client::MattermostClient;
use crate::config::MattermostConfigExt;
use crate::tools::build_mattermost_tools;

pub use assistant_core::MattermostConfig;

/// Mattermost interface runner. Connects via WebSocket and dispatches messages.
pub struct MattermostInterface {
    config: MattermostConfig,
    orchestrator: Arc<Orchestrator>,
}

impl MattermostInterface {
    pub fn new(config: MattermostConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
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
        let adapter = MattermostAdapter::new(
            client.clone(),
            self.config.allowed_channels.clone(),
            self.config.allowed_users.clone(),
        );

        // BOOT hook.
        let boot_id = Uuid::new_v4();
        if let Err(e) = self
            .orchestrator
            .run_boot(boot_id, Interface::Mattermost)
            .await
        {
            warn!(error = %e, "Mattermost BOOT hook failed (non-fatal)");
        }

        // Per-conversation serialisation.
        let conv_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let conversations: Arc<Mutex<LruCache<(String, String), Uuid>>> = Arc::new(Mutex::new(
            LruCache::new(NonZeroUsize::new(10_000).unwrap()),
        ));

        // Graceful shutdown.
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = sigterm.recv() => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            info!("Mattermost: shutdown signal received");
            let _ = shutdown_tx.send(true);
        });

        let mut stream = adapter.start().await?;
        info!("Mattermost: connected and listening");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Mattermost: shutting down");
                        break;
                    }
                }
                msg = stream.next() => {
                    match msg {
                        None => {
                            warn!("Mattermost: message stream ended");
                            break;
                        }
                        Some(channel_msg) => {
                            let conv_id = get_or_create_conversation(
                                &channel_msg,
                                &conversations,
                            ).await;

                            let lock = {
                                let mut locks = conv_locks.lock().await;
                                locks
                                    .entry(conv_id)
                                    .or_insert_with(|| Arc::new(Mutex::new(())))
                                    .clone()
                            };

                            let orchestrator = self.orchestrator.clone();
                            let api = client.clone();

                            tokio::spawn(async move {
                                let _guard = lock.lock().await;
                                dispatch_message(channel_msg, conv_id, orchestrator, api).await;
                            });
                        }
                    }
                }
            }
        }

        adapter.stop().await?;
        Ok(())
    }
}

async fn get_or_create_conversation(
    msg: &ChannelMessage,
    conversations: &Arc<Mutex<LruCache<(String, String), Uuid>>>,
) -> Uuid {
    let channel_id = msg
        .metadata
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let thread_id = msg
        .thread_id
        .clone()
        .unwrap_or_else(|| msg.platform_message_id.clone().unwrap_or_default());
    let key = (channel_id, thread_id);

    let mut convs = conversations.lock().await;
    if let Some(&existing) = convs.get(&key) {
        return existing;
    }
    let conv_id = Uuid::new_v4();
    convs.put(key, conv_id);
    conv_id
}

async fn dispatch_message(
    msg: ChannelMessage,
    conv_id: Uuid,
    orchestrator: Arc<Orchestrator>,
    client: Arc<MattermostClient>,
) {
    let text = match &msg.content {
        ChannelContent::Text(t) => t.clone(),
        _ => return,
    };

    let channel_id = msg
        .metadata
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let post_id = msg.platform_message_id.clone().unwrap_or_default();
    let thread_id = msg.thread_id.clone();
    let bot_user_id = client.token.clone(); // placeholder — real ID resolved at start()

    // Fetch bot ID for reaction tools (best-effort).
    let bot_user_id = match client.get_me().await {
        Ok(me) => me.id,
        Err(_) => bot_user_id,
    };

    let tools = build_mattermost_tools(
        channel_id.clone(),
        post_id.clone(),
        thread_id.clone(),
        bot_user_id,
        client.clone(),
    );

    match orchestrator
        .run_turn_with_tools(&text, conv_id, Interface::Mattermost, tools, None, vec![])
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(error = %e, conv_id = %conv_id, "Mattermost: orchestrator error");
            let _ = client
                .create_post(
                    &channel_id,
                    &format!("Sorry, I encountered an error: {e}"),
                    thread_id.as_deref(),
                )
                .await;
        }
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
    use assistant_core::MattermostConfig;

    #[test]
    fn allowlist_channel_logic_empty_accepts_all() {
        let cfg = MattermostConfig {
            allowed_channels: vec![],
            ..Default::default()
        };
        let channel = "town-square".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&channel);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_channel_logic_non_empty_blocks_unknown() {
        let cfg = MattermostConfig {
            allowed_channels: vec!["bot-test".to_string()],
            ..Default::default()
        };
        let unknown = "town-square".to_string();
        let blocked = !cfg.allowed_channels.is_empty() && !cfg.allowed_channels.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_user_logic_non_empty_passes_known() {
        let cfg = MattermostConfig {
            allowed_users: vec!["alice".to_string()],
            ..Default::default()
        };
        let known = "alice".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&known);
        assert!(!blocked);
    }
}
