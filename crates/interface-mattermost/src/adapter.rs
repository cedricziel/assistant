//! Mattermost `ChannelAdapter` implementation.
//!
//! Connects to the Mattermost WebSocket API, authenticates with a token
//! challenge, and yields inbound `posted` events as [`ChannelMessage`]s.
//! Automatic exponential-backoff reconnection is built in.

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
use futures::SinkExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::client::MattermostClient;

const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Mattermost `ChannelAdapter`. Connects via WebSocket and posts via REST.
pub struct MattermostAdapter {
    client: Arc<MattermostClient>,
    allowed_channels: Vec<String>,
    allowed_users: Vec<String>,
    /// Resolved bot user ID (set after a successful `get_me()` call in `start()`).
    bot_user_id: Arc<tokio::sync::RwLock<Option<String>>>,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
}

impl MattermostAdapter {
    pub fn new(
        client: Arc<MattermostClient>,
        allowed_channels: Vec<String>,
        allowed_users: Vec<String>,
    ) -> Self {
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        Self {
            client,
            allowed_channels,
            allowed_users,
            bot_user_id: Arc::new(tokio::sync::RwLock::new(None)),
            stop_tx,
            stop_rx,
        }
    }

    pub fn api_client(&self) -> Arc<MattermostClient> {
        self.client.clone()
    }
}

#[async_trait]
impl ChannelAdapter for MattermostAdapter {
    fn name(&self) -> &str {
        "mattermost"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Mattermost
    }

    async fn start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
        let client = self.client.clone();
        let allowed_channels = self.allowed_channels.clone();
        let allowed_users = self.allowed_users.clone();
        let mut stop_rx = self.stop_rx.clone();
        let bot_user_id_store = self.bot_user_id.clone();

        // Fetch bot's own user ID so we can filter self-messages.
        // Treat failure as non-fatal: self-message filtering is best-effort.
        let bot_user_id = match client.get_me().await {
            Ok(me) => {
                info!(user_id = %me.id, "Mattermost: resolved bot user ID");
                let uid = me.id.clone();
                *bot_user_id_store.write().await = Some(me.id);
                uid
            }
            Err(e) => {
                warn!(error = %e, "Mattermost: get_me() failed; self-message filtering disabled");
                String::new()
            }
        };

        let (tx, rx) = mpsc::channel::<ChannelMessage>(64);

        tokio::spawn(async move {
            let mut backoff = BACKOFF_MIN;
            loop {
                if *stop_rx.borrow() {
                    break;
                }

                let ws_url = client.ws_url();
                let ws_stream = match connect_async(&ws_url).await {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        error!(error = %e, "Mattermost WS connect failed; retrying");
                        sleep_backoff(&mut backoff, &mut stop_rx).await;
                        continue;
                    }
                };

                info!("Mattermost WebSocket connected");
                backoff = BACKOFF_MIN;

                let (mut ws_write, mut ws_read) = futures::StreamExt::split(ws_stream);

                // Send auth challenge.
                let auth = serde_json::json!({
                    "seq": 1,
                    "action": "authentication_challenge",
                    "data": { "token": client.token }
                })
                .to_string();
                if ws_write.send(WsMessage::Text(auth.into())).await.is_err() {
                    warn!("Mattermost WS auth send failed; reconnecting");
                    sleep_backoff(&mut backoff, &mut stop_rx).await;
                    continue;
                }

                loop {
                    tokio::select! {
                        _ = stop_rx.changed() => {
                            if *stop_rx.borrow() { break; }
                        }
                        msg = futures::StreamExt::next(&mut ws_read) => {
                            match msg {
                                None => {
                                    warn!("Mattermost WS stream ended; reconnecting");
                                    break;
                                }
                                Some(Err(e)) => {
                                    warn!(error = %e, "Mattermost WS error; reconnecting");
                                    break;
                                }
                                Some(Ok(WsMessage::Text(text))) => {
                                    let payload: serde_json::Value = match serde_json::from_str(&text) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            debug!(error = %e, "MM WS: JSON parse error");
                                            continue;
                                        }
                                    };

                                    // Only handle `posted` events.
                                    let event_type = payload.get("event").and_then(|v| v.as_str()).unwrap_or("");
                                    if event_type != "posted" {
                                        debug!(event = event_type, "MM WS: non-posted event");
                                        continue;
                                    }

                                    if let Some(msg) = parse_posted_event(
                                        &payload,
                                        &bot_user_id,
                                        &allowed_channels,
                                        &allowed_users,
                                    ) {
                                        if tx.send(msg).await.is_err() {
                                            break; // receiver dropped
                                        }
                                    }
                                }
                                Some(Ok(WsMessage::Ping(data))) => {
                                    let _ = ws_write.send(WsMessage::Pong(data)).await;
                                }
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }

                if *stop_rx.borrow() {
                    break;
                }
                sleep_backoff(&mut backoff, &mut stop_rx).await;
            }
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn send(&self, user: &ChannelUser, content: ChannelContent) -> Result<()> {
        let (channel_id, root_id) = parse_platform_id(&user.platform_id);
        if let ChannelContent::Text(text) = content {
            self.client
                .create_post(&channel_id, &text, root_id.as_deref())
                .await?;
        }
        Ok(())
    }

    async fn send_in_thread(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
        thread_id: &str,
    ) -> Result<()> {
        let (channel_id, _) = parse_platform_id(&user.platform_id);
        if let ChannelContent::Text(text) = content {
            self.client
                .create_post(&channel_id, &text, Some(thread_id))
                .await?;
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }

    fn platform_tools(&self, msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>> {
        let channel_id = msg
            .metadata
            .get("channel_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let post_id = msg.platform_message_id.clone().unwrap_or_default();
        let thread_id = msg.thread_id.clone();
        // Use the resolved bot user ID; fall back to empty string if not yet known.
        let bot_user_id = self
            .bot_user_id
            .try_read()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default();
        crate::tools::build_mattermost_tools(
            channel_id,
            post_id,
            thread_id,
            bot_user_id,
            self.client.clone(),
        )
    }

    /// Add ⏳ hourglass reaction immediately on message receipt (before the conv lock).
    async fn on_message_received(&self, msg: &ChannelMessage) -> Result<()> {
        let post_id = match &msg.platform_message_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => return Ok(()),
        };
        let bot_user_id = self
            .bot_user_id
            .try_read()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default();
        if bot_user_id.is_empty() {
            return Ok(());
        }
        if let Err(e) = self
            .client
            .add_reaction(&bot_user_id, &post_id, "hourglass_flowing_sand")
            .await
        {
            debug!(error = %e, "mattermost: failed to add hourglass reaction (best-effort)");
        }
        Ok(())
    }

    /// Remove ⏳ and send typing event when a turn starts.
    async fn on_turn_start(&self, user: &ChannelUser, _conv_id: Uuid) -> Result<()> {
        let (channel_id, thread_id) = parse_platform_id(&user.platform_id);
        let post_id = thread_id.as_deref().unwrap_or(&channel_id).to_string();
        let bot_user_id = self
            .bot_user_id
            .try_read()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default();
        if !bot_user_id.is_empty() && !post_id.is_empty() {
            let _ = self
                .client
                .remove_reaction(&bot_user_id, &post_id, "hourglass_flowing_sand")
                .await;
        }
        let _ = self.client.send_typing(&channel_id).await;
        Ok(())
    }

    async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()> {
        let (channel_id, thread_id) = parse_platform_id(&user.platform_id);
        let _ = self
            .client
            .create_post(
                &channel_id,
                &format!("Sorry, I encountered an error: {err}"),
                thread_id.as_deref(),
            )
            .await;
        Ok(())
    }
}

// -- Helpers ------------------------------------------------------------------

fn parse_platform_id(platform_id: &str) -> (String, Option<String>) {
    if let Some((channel, root)) = platform_id.split_once('/') {
        (channel.to_string(), Some(root.to_string()))
    } else {
        (platform_id.to_string(), None)
    }
}

/// Parse a `posted` WebSocket event into a [`ChannelMessage`], or `None` if
/// it should be ignored.
fn parse_posted_event(
    payload: &serde_json::Value,
    bot_user_id: &str,
    allowed_channels: &[String],
    allowed_users: &[String],
) -> Option<ChannelMessage> {
    let channel_id = payload
        .pointer("/broadcast/channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // The post data is a JSON string nested inside `data.post`.
    let post_str = payload.pointer("/data/post").and_then(|v| v.as_str())?;
    let post: serde_json::Value = serde_json::from_str(post_str).ok()?;

    let user_id = post.get("user_id").and_then(|v| v.as_str())?.to_string();
    let post_id = post.get("id").and_then(|v| v.as_str())?.to_string();
    let text = post
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let root_id = post
        .get("root_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    if text.is_empty() {
        return None;
    }
    // Filter self-messages.
    if user_id == bot_user_id {
        debug!(user_id, "MM: ignoring self-message");
        return None;
    }
    // Allowlist checks.
    if !allowed_channels.is_empty() && !allowed_channels.contains(&channel_id) {
        debug!(
            channel = channel_id,
            "MM: channel not in allowlist; dropping"
        );
        return None;
    }
    if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
        debug!(user = user_id, "MM: user not in allowlist; dropping");
        return None;
    }

    // Thread root for replies.
    let thread_root = root_id.clone().unwrap_or_else(|| post_id.clone());

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "channel_id".to_string(),
        serde_json::Value::String(channel_id.clone()),
    );
    metadata.insert(
        "post_id".to_string(),
        serde_json::Value::String(post_id.clone()),
    );
    metadata.insert(
        "user_id".to_string(),
        serde_json::Value::String(user_id.clone()),
    );

    Some(ChannelMessage {
        channel_type: ChannelType::Mattermost,
        platform_message_id: Some(post_id.clone()),
        sender: ChannelUser {
            // Encode channel_id + thread_root so send_in_thread can route correctly.
            platform_id: format!("{channel_id}/{thread_root}"),
            display_name: None,
        },
        content: ChannelContent::Text(text),
        thread_id: Some(thread_root),
        timestamp: Utc::now(),
        metadata,
    })
}

async fn sleep_backoff(backoff: &mut Duration, stop_rx: &mut tokio::sync::watch::Receiver<bool>) {
    let jitter_pct = (rand_jitter() - 0.5) * 0.2;
    let secs = backoff.as_secs_f64() * (1.0 + jitter_pct);
    let sleep_dur = Duration::from_secs_f64(secs.max(0.1));
    tokio::select! {
        _ = tokio::time::sleep(sleep_dur) => {}
        _ = stop_rx.changed() => {}
    }
    *backoff = (*backoff * 2).min(BACKOFF_MAX);
}

fn rand_jitter() -> f64 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let v = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (v >> 33) as f64 / (u32::MAX as f64)
}
