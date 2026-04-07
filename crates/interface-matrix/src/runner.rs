//! Matrix interface runner.
//!
//! Authenticates with a Matrix homeserver and drives the long-poll sync loop
//! via [`MatrixAdapter`].  Each inbound room message is dispatched to the
//! orchestrator with per-room conversation serialisation.

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

use crate::adapter::MatrixAdapter;
use crate::client::MatrixClient;
use crate::config::MatrixConfigExt;
use crate::tools::build_matrix_tools;

pub use assistant_core::MatrixConfig;

/// Matrix interface runner.
pub struct MatrixInterface {
    config: MatrixConfig,
    orchestrator: Arc<Orchestrator>,
}

impl MatrixInterface {
    pub fn new(config: MatrixConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
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
            info!("Matrix: using access token auth");
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
            info!(username = %username, "Matrix: logging in with password");
            MatrixClient::login(&homeserver_url, &username, &password).await?
        });

        info!(user_id = %client.user_id, "Matrix: authenticated");

        let adapter = MatrixAdapter::new(
            client.clone(),
            self.config.allowed_rooms.clone(),
            self.config.allowed_users.clone(),
        );

        // BOOT hook.
        let boot_id = Uuid::new_v4();
        if let Err(e) = self.orchestrator.run_boot(boot_id, Interface::Matrix).await {
            warn!(error = %e, "Matrix BOOT hook failed (non-fatal)");
        }

        // Per-room conversation serialisation.
        let conv_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let conversations: Arc<Mutex<LruCache<String, Uuid>>> = Arc::new(Mutex::new(
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
            info!("Matrix: shutdown signal received");
            let _ = shutdown_tx.send(true);
        });

        let mut stream = adapter.start().await?;
        info!("Matrix: sync loop started");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Matrix: shutting down");
                        break;
                    }
                }
                msg = stream.next() => {
                    match msg {
                        None => {
                            warn!("Matrix: message stream ended");
                            break;
                        }
                        Some(channel_msg) => {
                            let room_id = channel_msg
                                .metadata
                                .get("room_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            let conv_id = {
                                let mut convs = conversations.lock().await;
                                if let Some(&id) = convs.get(&room_id) {
                                    id
                                } else {
                                    let id = Uuid::new_v4();
                                    convs.put(room_id.clone(), id);
                                    id
                                }
                            };

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

async fn dispatch_message(
    msg: ChannelMessage,
    conv_id: Uuid,
    orchestrator: Arc<Orchestrator>,
    client: Arc<MatrixClient>,
) {
    let text = match &msg.content {
        ChannelContent::Text(t) => t.clone(),
        _ => return,
    };
    let room_id = msg
        .metadata
        .get("room_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let tools = build_matrix_tools(room_id.clone(), client.clone());

    match orchestrator
        .run_turn_with_tools(&text, conv_id, Interface::Matrix, tools, None, vec![])
        .await
    {
        Ok(_) => {}
        Err(e) => {
            error!(error = %e, conv_id = %conv_id, "Matrix: orchestrator error");
            let _ = client
                .send_message(&room_id, &format!("Sorry, I encountered an error: {e}"))
                .await;
        }
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
    use assistant_core::MatrixConfig;

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
}
