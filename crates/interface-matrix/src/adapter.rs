//! Matrix `ChannelAdapter` implementation.
//!
//! Drives a long-poll `/sync` loop, yielding inbound `m.room.message` events
//! as [`ChannelMessage`]s.  The `next_batch` token is persisted to disk so
//! the bot does not replay old messages after a restart.
//!
//! Auto-accepts room invitations via `POST /join/<room_id>`.

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser, ToolHandler,
};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::Stream;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::client::MatrixClient;

/// Matrix `ChannelAdapter`.
pub struct MatrixAdapter {
    client: Arc<MatrixClient>,
    allowed_rooms: Vec<String>,
    allowed_users: Vec<String>,
    /// Path to persist the `next_batch` token.
    next_batch_path: PathBuf,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
}

impl MatrixAdapter {
    pub fn new(
        client: Arc<MatrixClient>,
        allowed_rooms: Vec<String>,
        allowed_users: Vec<String>,
    ) -> Self {
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        let safe_user = client.user_id.replace([':', '@', '.'], "_");
        let next_batch_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".assistant")
            .join(format!("matrix-next-batch-{safe_user}.txt"));
        Self {
            client,
            allowed_rooms,
            allowed_users,
            next_batch_path,
            stop_tx,
            stop_rx,
        }
    }

    pub fn api_client(&self) -> Arc<MatrixClient> {
        self.client.clone()
    }
}

#[async_trait]
impl ChannelAdapter for MatrixAdapter {
    fn name(&self) -> &str {
        "matrix"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Matrix
    }

    async fn start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
        let client = self.client.clone();
        let allowed_rooms = self.allowed_rooms.clone();
        let allowed_users = self.allowed_users.clone();
        let next_batch_path = self.next_batch_path.clone();
        let mut stop_rx = self.stop_rx.clone();

        let (tx, rx) = mpsc::channel::<ChannelMessage>(64);

        tokio::spawn(async move {
            // Load persisted next_batch token.
            let mut since: Option<String> = tokio::fs::read_to_string(&next_batch_path)
                .await
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            loop {
                if *stop_rx.borrow() {
                    break;
                }

                // Track whether this is the very first (bootstrap) sync so we
                // can seed the `since` token without emitting old messages.
                let is_bootstrap = since.is_none();

                let sync_result = tokio::select! {
                    r = client.sync(since.as_deref(), 30_000) => r,
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() { break; }
                        continue;
                    }
                };

                let sync_resp = match sync_result {
                    Ok(r) => r,
                    Err(e) => {
                        warn!(error = %e, "Matrix sync failed; retrying in 5s");
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                            _ = stop_rx.changed() => {}
                        }
                        continue;
                    }
                };

                let next = sync_resp.next_batch.clone();

                // Handle room invites — auto-join (respecting the allowlist).
                if let Some(rooms) = &sync_resp.rooms {
                    if let Some(invites) = &rooms.invite {
                        for room_id in invites.keys() {
                            if !allowed_rooms.is_empty() && !allowed_rooms.contains(room_id) {
                                debug!(
                                    room_id,
                                    "Matrix: ignoring invite from room not in allowlist"
                                );
                                continue;
                            }
                            info!(room_id, "Matrix: auto-joining invited room");
                            if let Err(e) = client.join_room(room_id).await {
                                warn!(error = %e, room_id, "Matrix: failed to join room");
                            }
                        }
                    }

                    // Skip processing events from the bootstrap sync — they
                    // represent historical state and should not be dispatched.
                    if is_bootstrap {
                        // Fall through to persist next_batch, then continue.
                    } else
                    // Process joined room timeline events.
                    if let Some(joined) = &rooms.join {
                        for (room_id, joined_room) in joined {
                            let Some(timeline) = &joined_room.timeline else {
                                continue;
                            };
                            for event in &timeline.events {
                                if event.event_type != "m.room.message" {
                                    continue;
                                }
                                let msgtype = event
                                    .content
                                    .get("msgtype")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                if msgtype != "m.text" {
                                    continue;
                                }
                                let text = event
                                    .content
                                    .get("body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if text.is_empty() {
                                    continue;
                                }
                                // Filter own messages.
                                if event.sender == client.user_id {
                                    debug!(sender = %event.sender, "Matrix: ignoring self-message");
                                    continue;
                                }
                                // Allowlist checks.
                                if !allowed_rooms.is_empty() && !allowed_rooms.contains(room_id) {
                                    debug!(room_id, "Matrix: room not in allowlist; dropping");
                                    continue;
                                }
                                if !allowed_users.is_empty()
                                    && !allowed_users.contains(&event.sender)
                                {
                                    debug!(sender = %event.sender, "Matrix: user not in allowlist; dropping");
                                    continue;
                                }

                                let mut metadata = std::collections::HashMap::new();
                                metadata.insert(
                                    "room_id".to_string(),
                                    serde_json::Value::String(room_id.clone()),
                                );
                                metadata.insert(
                                    "sender".to_string(),
                                    serde_json::Value::String(event.sender.clone()),
                                );

                                let msg = ChannelMessage {
                                    channel_type: ChannelType::Matrix,
                                    platform_message_id: event.event_id.clone(),
                                    sender: ChannelUser {
                                        platform_id: room_id.clone(),
                                        display_name: Some(event.sender.clone()),
                                    },
                                    content: ChannelContent::Text(text),
                                    thread_id: None,
                                    timestamp: Utc::now(),
                                    metadata,
                                };

                                if tx.send(msg).await.is_err() {
                                    return; // receiver dropped
                                }
                            }
                        }
                    }
                }

                // Persist next_batch token.
                since = Some(next.clone());
                if let Some(parent) = next_batch_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&next_batch_path, &next).await;
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn send(&self, user: &ChannelUser, content: ChannelContent) -> Result<()> {
        if let ChannelContent::Text(text) = content {
            self.client.send_message(&user.platform_id, &text).await?;
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }

    /// Matrix conversations are per-room; use room_id as the key.
    fn conversation_key(&self, msg: &ChannelMessage) -> String {
        msg.metadata
            .get("room_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&msg.sender.platform_id)
            .to_string()
    }

    fn platform_tools(&self, msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>> {
        let room_id = msg
            .metadata
            .get("room_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        crate::tools::build_matrix_tools(room_id, self.client.clone())
    }

    async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()> {
        let room_id = &user.platform_id;
        let _ = self
            .client
            .send_message(room_id, &format!("Sorry, I encountered an error: {err}"))
            .await;
        Ok(())
    }
}
