//! Slack `ChannelAdapter` implementation.
//!
//! Opens a Slack Socket Mode WebSocket connection using plain
//! `tokio-tungstenite` + `reqwest`, with automatic exponential-backoff
//! reconnection.  Inbound events are yielded as [`ChannelMessage`]s via a
//! `futures::Stream`.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser, SlackConfig,
};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::Stream;
use futures::SinkExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

use crate::client::SlackApiClient;
use crate::config::SlackConfigExt;

/// Minimum reconnect delay.
const BACKOFF_MIN: Duration = Duration::from_secs(1);
/// Maximum reconnect delay.
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Slack `ChannelAdapter`.  Connects via Socket Mode WebSocket and provides
/// `send()` / `send_in_thread()` via `chat.postMessage`.
pub struct SlackAdapter {
    config: SlackConfig,
    client: Arc<SlackApiClient>,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
}

impl SlackAdapter {
    /// Create from config.  Validates that both tokens are present.
    pub fn new(config: SlackConfig) -> Result<Self> {
        let bot_token = config
            .resolved_bot_token()
            .ok_or_else(|| anyhow::anyhow!("SLACK_BOT_TOKEN not configured"))?;
        let app_token = config
            .resolved_app_token()
            .ok_or_else(|| anyhow::anyhow!("SLACK_APP_TOKEN not configured"))?;
        let client = Arc::new(SlackApiClient::new(bot_token, app_token)?);
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        Ok(Self {
            config,
            client,
            stop_tx,
            stop_rx,
        })
    }

    /// Expose the underlying API client (for tools / skills).
    pub fn api_client(&self) -> Arc<SlackApiClient> {
        self.client.clone()
    }
}

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    fn name(&self) -> &str {
        "slack"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Slack
    }

    async fn start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
        let client = self.client.clone();
        let allowed_channels = self.config.allowed_channels.clone();
        let allowed_users = self.config.allowed_users.clone();
        let mut stop_rx = self.stop_rx.clone();

        let (tx, rx) = mpsc::channel::<ChannelMessage>(64);

        tokio::spawn(async move {
            let mut backoff = BACKOFF_MIN;
            loop {
                // Check stop signal.
                if *stop_rx.borrow() {
                    break;
                }

                // Get a fresh WebSocket URL on each connection attempt.
                let ws_url = match client.apps_connections_open().await {
                    Ok(url) => url,
                    Err(e) => {
                        error!(error = %e, "apps.connections.open failed; retrying");
                        sleep_backoff(&mut backoff, &mut stop_rx).await;
                        continue;
                    }
                };

                let ws_stream = match connect_async(&ws_url).await {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        error!(error = %e, "WebSocket connect failed; retrying");
                        sleep_backoff(&mut backoff, &mut stop_rx).await;
                        continue;
                    }
                };

                info!("Slack Socket Mode connected");
                backoff = BACKOFF_MIN; // reset on success

                let (mut ws_write, mut ws_read) = futures::StreamExt::split(ws_stream);

                loop {
                    tokio::select! {
                        _ = stop_rx.changed() => {
                            if *stop_rx.borrow() {
                                break;
                            }
                        }
                        msg = futures::StreamExt::next(&mut ws_read) => {
                            match msg {
                                None => {
                                    warn!("Slack WS stream ended; reconnecting");
                                    break;
                                }
                                Some(Err(e)) => {
                                    warn!(error = %e, "Slack WS error; reconnecting");
                                    break;
                                }
                                Some(Ok(WsMessage::Text(text))) => {
                                    let payload: serde_json::Value =
                                        match serde_json::from_str(&text) {
                                            Ok(v) => v,
                                            Err(e) => {
                                                debug!(error = %e, "Slack WS: JSON parse error");
                                                continue;
                                            }
                                        };

                                    // Ack every envelope that has an envelope_id.
                                    if let Some(env_id) =
                                        payload.get("envelope_id").and_then(|v| v.as_str())
                                    {
                                        let ack = serde_json::json!({
                                            "envelope_id": env_id,
                                            "type": "ack"
                                        })
                                        .to_string();
                                        let _ = ws_write.send(WsMessage::Text(ack.into())).await;
                                    }

                                    // Skip non-event frames (hello, disconnect, etc.)
                                    let event_type = payload
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    if event_type != "events_api" {
                                        debug!(type_ = event_type, "Slack WS: non-event frame");
                                        continue;
                                    }

                                    // Navigate to the inner event payload.
                                    let inner = match payload
                                        .pointer("/payload/event")
                                        .or_else(|| payload.pointer("/event"))
                                    {
                                        Some(e) => e.clone(),
                                        None => continue,
                                    };

                                    if let Some(msg) = parse_event(
                                        &inner,
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
        let (channel, thread_ts) = parse_platform_id(&user.platform_id);
        if let ChannelContent::Text(text) = content {
            self.client
                .post_message(&channel, &text, thread_ts.as_deref())
                .await?;
        }
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }

    async fn send_reaction(
        &self,
        user: &ChannelUser,
        message_id: &str,
        reaction: &str,
    ) -> Result<()> {
        let (channel, _) = parse_platform_id(&user.platform_id);
        self.client
            .add_reaction(&channel, message_id, reaction)
            .await
    }

    async fn send_in_thread(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
        thread_id: &str,
    ) -> Result<()> {
        let (channel, _) = parse_platform_id(&user.platform_id);
        if let ChannelContent::Text(text) = content {
            self.client
                .post_message(&channel, &text, Some(thread_id))
                .await?;
        }
        Ok(())
    }
}

// -- Helpers ------------------------------------------------------------------

/// Parse a `platform_id` encoded as `"<channel_id>[/<thread_ts>]"`.
fn parse_platform_id(platform_id: &str) -> (String, Option<String>) {
    if let Some((channel, thread)) = platform_id.split_once('/') {
        (channel.to_string(), Some(thread.to_string()))
    } else {
        (platform_id.to_string(), None)
    }
}

/// Convert a Slack Socket Mode event payload to a [`ChannelMessage`], or
/// return `None` if the event should be ignored.
fn parse_event(
    event: &serde_json::Value,
    allowed_channels: &[String],
    allowed_users: &[String],
) -> Option<ChannelMessage> {
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

    // Only handle plain message events.
    if event_type != "message" {
        return None;
    }

    // Skip bot messages and subtypes (message_changed, etc.)
    if event.get("bot_id").is_some() {
        return None;
    }
    if let Some(subtype) = event.get("subtype").and_then(|v| v.as_str()) {
        if subtype != "file_share" {
            return None;
        }
    }

    let channel_id = event.get("channel").and_then(|v| v.as_str())?.to_string();
    let user_id = event.get("user").and_then(|v| v.as_str())?.to_string();
    let text = event
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let ts = event
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let thread_ts = event
        .get("thread_ts")
        .and_then(|v| v.as_str())
        .unwrap_or(&ts)
        .to_string();

    // Allowlist filtering.
    if !allowed_channels.is_empty() && !allowed_channels.contains(&channel_id) {
        debug!(
            channel = channel_id,
            "Slack: channel not in allowlist; dropping"
        );
        return None;
    }
    if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
        debug!(user = user_id, "Slack: user not in allowlist; dropping");
        return None;
    }

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "channel_id".to_string(),
        serde_json::Value::String(channel_id.clone()),
    );
    metadata.insert("ts".to_string(), serde_json::Value::String(ts.clone()));

    Some(ChannelMessage {
        channel_type: ChannelType::Slack,
        platform_message_id: Some(ts),
        sender: ChannelUser {
            // Encode channel_id + thread_ts so send_in_thread can route correctly.
            platform_id: format!("{channel_id}/{thread_ts}"),
            display_name: None,
        },
        content: ChannelContent::Text(text),
        thread_id: Some(thread_ts),
        timestamp: Utc::now(),
        metadata,
    })
}

/// Sleep for the current backoff duration (with ±10% jitter), then double it.
async fn sleep_backoff(backoff: &mut Duration, stop_rx: &mut tokio::sync::watch::Receiver<bool>) {
    use std::time::Duration;
    // Add ±10% jitter.
    let jitter_pct = (rand_jitter() - 0.5) * 0.2; // -10% to +10%
    let secs = backoff.as_secs_f64() * (1.0 + jitter_pct);
    let sleep_dur = Duration::from_secs_f64(secs.max(0.1));

    tokio::select! {
        _ = tokio::time::sleep(sleep_dur) => {}
        _ = stop_rx.changed() => {}
    }

    // Exponential increase, capped.
    *backoff = (*backoff * 2).min(BACKOFF_MAX);
}

fn rand_jitter() -> f64 {
    // Simple LCG-based float in [0,1) without pulling in `rand`.
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
