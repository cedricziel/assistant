//! Slack `ChannelAdapter` implementation.
//!
//! Opens a Slack Socket Mode WebSocket connection using plain
//! `tokio-tungstenite` + `reqwest`, with automatic exponential-backoff
//! reconnection.  Inbound events are yielded as [`ChannelMessage`]s via a
//! `futures::Stream`.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::types::channels::{SlackConfig, SlackListenMode};
use assistant_core::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser, Message, MessageRole,
    ToolHandler,
};
use assistant_storage::StorageLayer;
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest, is_audio_mime};
use async_trait::async_trait;
use chrono::Utc;
use futures::SinkExt;
use futures::stream::Stream;
use lru::LruCache;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::client::SlackApiClient;
use super::config::SlackConfigExt;

use crate::common::{BACKOFF_MIN, sleep_backoff};

/// Maximum audio file size downloaded for transcription (25 MB — Whisper API limit).
/// LRU capacity for the in-process cache of Slack thread roots that the bot
/// has been @-mentioned in. Compile-time-evaluated so the non-zero invariant
/// cannot regress.
const ACTIVE_THREAD_CACHE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(1024) {
    Some(n) => n,
    None => unreachable!(),
};

const MAX_AUDIO_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
/// Maximum image file size downloaded for vision (10 MB).
const MAX_IMAGE_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;

/// Slack `ChannelAdapter`.  Connects via Socket Mode WebSocket and provides
/// `send()` / `send_in_thread()` via `chat.postMessage`.
pub struct SlackAdapter {
    config: SlackConfig,
    client: Arc<SlackApiClient>,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    /// Optional storage layer for seeding thread history on first turn.
    storage: Option<Arc<StorageLayer>>,
    /// Set of `platform_id`s for which history has already been seeded.
    seeded_keys: Arc<Mutex<HashSet<String>>>,
    /// Optional audio transcription provider for voice messages.
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    /// BCP-47 language hint passed to the transcription provider.
    transcription_language: Option<String>,
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
            storage: None,
            seeded_keys: Arc::new(Mutex::new(HashSet::new())),
            transcription: None,
            transcription_language: None,
        })
    }

    /// Attach a storage layer to enable thread-history seeding on first turn.
    pub fn with_storage(mut self, storage: Arc<StorageLayer>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Attach an audio transcription provider for voice message support.
    pub fn with_transcription(
        mut self,
        provider: Arc<dyn TranscriptionProvider>,
        language: Option<String>,
    ) -> Self {
        self.transcription = Some(provider);
        self.transcription_language = language;
        self
    }

    /// Expose the underlying API client (for tools / skills).
    #[allow(dead_code)]
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
        let listen_mode = self.config.mode.clone();
        let storage = self.storage.clone();
        let mut stop_rx = self.stop_rx.clone();
        let transcription = self.transcription.clone();
        let transcription_language = self.transcription_language.clone();

        // Resolve the bot's own Slack user ID so we can detect @-mentions
        // and filter self-messages.
        let bot_user_id = match client.auth_test().await {
            Ok(id) => {
                info!(bot_user_id = %id, mode = ?listen_mode, "Slack listen mode configured");
                id
            }
            Err(e) => {
                warn!(error = %e, "auth.test failed; falling back to empty bot_user_id (mention filtering will not work)");
                String::new()
            }
        };

        let (tx, rx) = mpsc::channel::<ChannelMessage>(64);

        tokio::spawn(async move {
            // In-memory LRU cache for active threads (bot was @-mentioned).
            // Survives reconnects within the same process; DB provides
            // persistence across restarts.
            let mut active_threads: LruCache<String, ()> =
                LruCache::new(ACTIVE_THREAD_CACHE_CAPACITY);

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

                                    if let Some(mut msg) = parse_event(
                                        &inner,
                                        &allowed_channels,
                                        &allowed_users,
                                    ) {
                                        // Filter self-messages by user ID.
                                        let sender_user_id = inner
                                            .get("user")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        if !bot_user_id.is_empty()
                                            && sender_user_id == bot_user_id
                                        {
                                            debug!("Slack: ignoring self-message");
                                            continue;
                                        }

                                        // Apply listen-mode filtering.
                                        let channel_id = inner
                                            .get("channel")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        let msg_text = inner
                                            .get("text")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        let ts_val = inner
                                            .get("ts")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        let thread_ts_val = inner
                                            .get("thread_ts")
                                            .and_then(|v| v.as_str());

                                        // Build the thread key for cache/DB lookups.
                                        let thread_key = format!(
                                            "{}/{}",
                                            channel_id,
                                            thread_ts_val.unwrap_or(ts_val)
                                        );

                                        // Check if this thread is already tracked
                                        // (LRU first, then DB fallback).
                                        let is_tracked =
                                            active_threads.contains(&thread_key) || {
                                                let db_tracked = if let Some(ref st) = storage {
                                                    st.slack_active_thread_store()
                                                        .contains(
                                                            channel_id,
                                                            thread_ts_val.unwrap_or(ts_val),
                                                        )
                                                        .await
                                                        .unwrap_or(false)
                                                } else {
                                                    false
                                                };
                                                if db_tracked {
                                                    // Warm the LRU cache from DB.
                                                    active_threads
                                                        .put(thread_key.clone(), ());
                                                }
                                                db_tracked
                                            };

                                        let decision = should_process(
                                            &listen_mode,
                                            &bot_user_id,
                                            channel_id,
                                            msg_text,
                                            thread_ts_val,
                                            ts_val,
                                            is_tracked,
                                        );

                                        match decision {
                                            ShouldProcessResult::Reject => {
                                                debug!(
                                                    channel = channel_id,
                                                    mode = ?listen_mode,
                                                    "Slack: message filtered by listen mode"
                                                );
                                                continue;
                                            }
                                            ShouldProcessResult::AcceptAndTrack => {
                                                // Track this thread so future replies
                                                // are also accepted.
                                                active_threads
                                                    .put(thread_key.clone(), ());
                                                if let Some(ref st) = storage
                                                    && let Err(e) = st
                                                        .slack_active_thread_store()
                                                        .upsert(
                                                            channel_id,
                                                            thread_ts_val.unwrap_or(ts_val),
                                                        )
                                                        .await
                                                {
                                                    warn!(
                                                        error = %e,
                                                        "failed to persist active thread"
                                                    );
                                                }
                                            }
                                            ShouldProcessResult::Accept => {}
                                        }

                                        // Transcribe audio attachments on file_share events.
                                        if let Some(ref provider) = transcription {
                                            let transcript = transcribe_audio_from_event(
                                                &inner,
                                                &client,
                                                provider.as_ref(),
                                                transcription_language.as_deref(),
                                            )
                                            .await;
                                            if !transcript.is_empty()
                                                && let ChannelContent::Text(ref mut text) =
                                                    msg.content
                                                {
                                                    if text.is_empty() {
                                                        *text = transcript;
                                                    } else {
                                                        *text =
                                                            format!("{transcript}\n{text}");
                                                    }
                                                }
                                        }

                                        // Download image attachments from file_share events.
                                        if let Some(image_content) =
                                            extract_image_from_event(&inner, &client).await
                                        {
                                            // Preserve any text (caption) from the original message.
                                            let caption = match &msg.content {
                                                ChannelContent::Text(t) if !t.is_empty() => {
                                                    Some(t.clone())
                                                }
                                                _ => None,
                                            };
                                            msg.content = image_content;

                                            // If there was a caption, send it as a separate text
                                            // message before the image so the LLM sees both.
                                            if let Some(cap) = caption {
                                                let text_msg = ChannelMessage {
                                                    content: ChannelContent::Text(cap),
                                                    ..msg.clone()
                                                };
                                                if tx.send(text_msg).await.is_err() {
                                                    break;
                                                }
                                            }
                                        }

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
        match content {
            ChannelContent::Text(text) => {
                self.client
                    .post_message(&channel, &text, thread_ts.as_deref())
                    .await?;
            }
            ChannelContent::FileData {
                data,
                filename,
                mime_type: _,
            } => {
                self.client
                    .upload_file(&channel, &filename, data, thread_ts.as_deref())
                    .await?;
            }
            _ => {}
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
        match content {
            ChannelContent::Text(text) => {
                self.client
                    .post_message(&channel, &text, Some(thread_id))
                    .await?;
            }
            ChannelContent::FileData {
                data,
                filename,
                mime_type: _,
            } => {
                self.client
                    .upload_file(&channel, &filename, data, Some(thread_id))
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn platform_tools(&self, msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>> {
        // Prefer the platform-native channel_id from metadata (inbound turns).
        // Fall back to sender.platform_id for synthetic messages (scheduler turns).
        let channel_id = msg
            .metadata
            .get("channel_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| msg.sender.platform_id.clone());
        let thread_ts = msg.thread_id.clone();
        let message_ts = msg
            .platform_message_id
            .clone()
            .unwrap_or_else(|| thread_ts.clone().unwrap_or_default());
        super::tools::build_slack_tools(channel_id, thread_ts, message_ts, self.client.clone())
            .into_iter()
            .map(Arc::from)
            .collect()
    }

    /// Add ⏳ reaction immediately when a message arrives (before the conv lock).
    async fn on_message_received(&self, msg: &ChannelMessage) -> Result<()> {
        let (channel, _) = parse_platform_id(&msg.sender.platform_id);
        if let Some(ts) = &msg.platform_message_id
            && !channel.is_empty()
            && !ts.is_empty()
        {
            let _ = self
                .client
                .add_reaction(&channel, ts, "hourglass_flowing_sand")
                .await;
        }
        Ok(())
    }

    /// Remove ⏳, add 👀, call setStatus, and seed thread history on the first turn.
    async fn on_turn_start(&self, user: &ChannelUser, conv_id: Uuid) -> Result<()> {
        let (channel, thread_ts) = parse_platform_id(&user.platform_id);

        if !channel.is_empty() {
            let ts = thread_ts.as_deref().unwrap_or("").to_string();
            if !ts.is_empty() {
                // Remove the queued ⏳ hourglass.
                let _ = self
                    .client
                    .remove_reaction(&channel, &ts, "hourglass_flowing_sand")
                    .await;
                // Add 👀 to signal active processing.
                let _ = self.client.add_reaction(&channel, &ts, "eyes").await;
                // Show animated agent status in the Slack assistant thread UI.
                let _ = self
                    .client
                    .set_agent_status(
                        &channel,
                        &ts,
                        "Working on it...",
                        &[
                            "Thinking...",
                            "Searching knowledge base...",
                            "Processing your request...",
                            "Almost there...",
                        ],
                    )
                    .await;
            }
        }

        // Seed thread history into the conversation store on first touch.
        if let Some(storage) = &self.storage {
            let platform_id = user.platform_id.clone();
            let already_seeded = {
                let mut seeded = self.seeded_keys.lock().await;
                !seeded.insert(platform_id.clone())
            };
            if !already_seeded {
                let thread_key = thread_ts.as_deref().unwrap_or("").to_string();
                if !channel.is_empty() && !thread_key.is_empty() {
                    match self
                        .client
                        .conversations_replies(&channel, &thread_key, 100)
                        .await
                    {
                        Ok(msgs) => {
                            let _ = seed_thread_history(conv_id, &msgs, storage).await;
                        }
                        Err(e) => {
                            debug!(error = %e, "slack: failed to fetch thread history for seeding");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Add ✅ reaction on successful turn.
    async fn on_turn_success(
        &self,
        user: &ChannelUser,
        _answer: &str,
        _attachments: &[assistant_core::Attachment],
    ) -> Result<()> {
        let (channel, thread_ts) = parse_platform_id(&user.platform_id);
        if !channel.is_empty() {
            let ts = thread_ts.unwrap_or_default();
            if !ts.is_empty() {
                let _ = self
                    .client
                    .add_reaction(&channel, &ts, "white_check_mark")
                    .await;
            }
        }
        Ok(())
    }

    /// Post an error message to the channel.
    async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()> {
        let (channel, thread_ts) = parse_platform_id(&user.platform_id);
        if !channel.is_empty() {
            let _ = self
                .client
                .post_message(
                    &channel,
                    &format!("Sorry, I encountered an error: {err}"),
                    thread_ts.as_deref(),
                )
                .await;
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
    if let Some(subtype) = event.get("subtype").and_then(|v| v.as_str())
        && subtype != "file_share"
    {
        return None;
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

/// Download and transcribe audio files attached to a Slack `file_share` event.
///
/// Returns a combined transcript string (one `[Voice transcription]` block per
/// audio file) to be prepended to the message text.  Returns an empty string
/// when no audio files are found or transcription is not possible.
async fn transcribe_audio_from_event(
    event: &serde_json::Value,
    client: &SlackApiClient,
    provider: &dyn TranscriptionProvider,
    language: Option<&str>,
) -> String {
    let files = match event.get("files").and_then(|f| f.as_array()) {
        Some(f) if !f.is_empty() => f.clone(),
        _ => return String::new(),
    };

    let mut transcripts = Vec::new();
    for file in &files {
        let mime = match file.get("mimetype").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => continue,
        };
        if !is_audio_mime(mime) {
            continue;
        }

        let url = file
            .get("url_private_download")
            .or_else(|| file.get("url_private"))
            .and_then(|v| v.as_str());
        let url = match url {
            Some(u) => u,
            None => continue,
        };
        let filename = file
            .get("name")
            .or_else(|| file.get("title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let audio_data = match client
            .download_private_file(url, MAX_AUDIO_ATTACHMENT_BYTES)
            .await
        {
            Ok(bytes) => bytes,
            Err(e) => {
                warn!(error = %e, "slack: failed to download audio file");
                continue;
            }
        };

        let request = TranscriptionRequest {
            audio_data,
            mime_type: mime.to_string(),
            filename,
            language: language.map(|s| s.to_string()),
        };

        match provider.transcribe(request).await {
            Ok(result) => {
                info!(
                    text_len = result.text.len(),
                    "slack: audio transcription successful"
                );
                transcripts.push(format!("[Voice transcription]: {}", result.text));
            }
            Err(e) => {
                warn!(error = %e, "slack: audio transcription failed");
                transcripts.push("[Voice message: transcription failed]".to_string());
            }
        }
    }

    transcripts.join("\n")
}

/// Download image files attached to a Slack `file_share` event.
///
/// Returns `Some(ChannelContent::FileData)` for the first image found,
/// or `None` when no images are present.
async fn extract_image_from_event(
    event: &serde_json::Value,
    client: &SlackApiClient,
) -> Option<ChannelContent> {
    let files = event.get("files").and_then(|f| f.as_array())?;

    for file in files {
        let mime = file.get("mimetype").and_then(|v| v.as_str())?;
        if !mime.starts_with("image/") {
            continue;
        }

        let url = file
            .get("url_private_download")
            .or_else(|| file.get("url_private"))
            .and_then(|v| v.as_str())?;

        let filename = file
            .get("name")
            .or_else(|| file.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("image")
            .to_string();

        match client
            .download_private_file(url, MAX_IMAGE_ATTACHMENT_BYTES)
            .await
        {
            Ok(bytes) => {
                info!(
                    filename = %filename,
                    size = bytes.len(),
                    mime = %mime,
                    "slack: downloaded image attachment"
                );
                return Some(ChannelContent::FileData {
                    data: bytes,
                    filename,
                    mime_type: mime.to_string(),
                });
            }
            Err(e) => {
                warn!(error = %e, "slack: failed to download image file");
                continue;
            }
        }
    }

    None
}

/// Seed Slack thread messages into the conversation store.
///
/// Called once per new thread the first time the adapter handles a message in
/// that thread.  Seeds both user and bot messages so the LLM has context from
/// prior exchanges.
async fn seed_thread_history(
    conv_id: Uuid,
    messages: &[serde_json::Value],
    storage: &Arc<StorageLayer>,
) -> Result<()> {
    for msg in messages {
        let subtype = msg.get("subtype").and_then(|v| v.as_str());
        if let Some(st) = subtype
            && st != "file_share"
        {
            continue;
        }

        let is_bot = msg.get("bot_id").is_some()
            || msg
                .get("display_as_bot")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let role = if is_bot {
            MessageRole::Assistant
        } else if msg.get("user").is_some() {
            MessageRole::User
        } else {
            continue;
        };

        let text = msg
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if text.is_empty() {
            continue;
        }

        let m = Message::new(conv_id, role, text);
        let _ = storage
            .conversation_store_for_agent("default")
            .save_message(&m)
            .await;
    }
    Ok(())
}

// -- Listen-mode filtering ----------------------------------------------------

/// Result of evaluating whether the bot should process a Slack message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShouldProcessResult {
    /// Process the message normally.
    Accept,
    /// Process the message and remember this thread (bot was @-mentioned).
    AcceptAndTrack,
    /// Skip the message.
    Reject,
}

/// Pure, sync filter function that decides whether a message should be processed
/// based on the configured [`SlackListenMode`].
///
/// `is_thread_tracked` should be `true` when the thread identified by `thread_ts`
/// is already in the active-thread cache (LRU or DB).
fn should_process(
    mode: &SlackListenMode,
    bot_user_id: &str,
    channel_id: &str,
    text: &str,
    thread_ts: Option<&str>,
    ts: &str,
    is_thread_tracked: bool,
) -> ShouldProcessResult {
    match mode {
        SlackListenMode::All => ShouldProcessResult::Accept,
        SlackListenMode::Mention => {
            // DMs (Slack DM channel IDs start with "D") are always accepted.
            if channel_id.starts_with('D') {
                return ShouldProcessResult::Accept;
            }

            // Messages containing an @-mention of the bot are always accepted
            // and the thread is tracked for future replies.
            let mention_tag = format!("<@{bot_user_id}>");
            if text.contains(&mention_tag) {
                return ShouldProcessResult::AcceptAndTrack;
            }

            // Thread replies in a tracked thread are accepted.
            if let Some(tts) = thread_ts
                && tts != ts
                && is_thread_tracked
            {
                return ShouldProcessResult::Accept;
            }

            ShouldProcessResult::Reject
        }
    }
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::slack::client::SlackApiClient;

    fn make_adapter(server: &MockServer) -> SlackAdapter {
        let client = Arc::new(
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap(),
        );
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        SlackAdapter {
            config: Default::default(),
            client,
            stop_tx,
            stop_rx,
            storage: None,
            seeded_keys: Arc::new(Mutex::new(Default::default())),
            transcription: None,
            transcription_language: None,
        }
    }

    fn make_msg(channel: &str, ts: &str) -> ChannelMessage {
        use assistant_core::{ChannelContent, ChannelType};
        use chrono::Utc;
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "channel_id".into(),
            serde_json::Value::String(channel.into()),
        );
        ChannelMessage {
            channel_type: ChannelType::Slack,
            platform_message_id: Some(ts.into()),
            sender: ChannelUser {
                platform_id: format!("{channel}/{ts}"),
                display_name: None,
            },
            content: ChannelContent::Text("hello".into()),
            thread_id: Some(ts.into()),
            timestamp: Utc::now(),
            metadata,
        }
    }

    #[tokio::test]
    async fn on_message_received_adds_hourglass() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;

        let adapter = make_adapter(&server);
        let msg = make_msg("C123", "111.222");
        adapter.on_message_received(&msg).await.unwrap();

        // wiremock will assert the mock was hit when server drops
    }

    #[tokio::test]
    async fn on_turn_start_removes_hourglass_adds_eyes_and_sets_status() {
        let server = MockServer::start().await;
        // reactions.remove (hourglass)
        Mock::given(method("POST"))
            .and(path("/api/reactions.remove"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;
        // reactions.add (eyes)
        Mock::given(method("POST"))
            .and(path("/api/reactions.add"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;
        // assistant.threads.setStatus
        Mock::given(method("POST"))
            .and(path("/api/assistant.threads.setStatus"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;

        let adapter = make_adapter(&server);
        let user = ChannelUser {
            platform_id: "C123/111.222".into(),
            display_name: None,
        };
        adapter
            .on_turn_start(&user, uuid::Uuid::new_v4())
            .await
            .unwrap();
    }

    #[test]
    fn parse_event_accepts_file_share_subtype() {
        let event = serde_json::json!({
            "type": "message",
            "subtype": "file_share",
            "channel": "C123",
            "user": "U456",
            "text": "check this out",
            "ts": "111.222",
            "files": [{"mimetype": "image/png", "name": "screenshot.png"}]
        });
        let msg = parse_event(&event, &[], &[]);
        assert!(msg.is_some(), "file_share subtype must be accepted");
        let msg = msg.unwrap();
        assert!(matches!(msg.content, ChannelContent::Text(ref t) if t == "check this out"));
    }

    #[tokio::test]
    async fn extract_image_downloads_image_file() {
        let server = MockServer::start().await;

        let png_bytes = b"fakepngdata";
        Mock::given(method("GET"))
            .and(path("/files/image.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(png_bytes.to_vec()))
            .mount(&server)
            .await;

        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        let event = serde_json::json!({
            "files": [{
                "mimetype": "image/png",
                "name": "screenshot.png",
                "url_private_download": format!("{}/files/image.png", server.uri())
            }]
        });

        let result = extract_image_from_event(&event, &client).await;
        assert!(result.is_some(), "should extract image from file_share");
        match result.unwrap() {
            ChannelContent::FileData {
                data,
                filename,
                mime_type,
            } => {
                assert_eq!(data, png_bytes);
                assert_eq!(filename, "screenshot.png");
                assert_eq!(mime_type, "image/png");
            }
            other => panic!("expected FileData, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_file_data_calls_upload_file() {
        let server = MockServer::start().await;

        // Step 1: files.getUploadURLExternal
        Mock::given(method("GET"))
            .and(path("/api/files.getUploadURLExternal"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "upload_url": format!("{}/upload-target", server.uri()),
                "file_id": "F123"
            })))
            .mount(&server)
            .await;

        // Step 2: upload target
        Mock::given(method("POST"))
            .and(path("/upload-target"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        // Step 3: files.completeUploadExternal
        Mock::given(method("POST"))
            .and(path("/api/files.completeUploadExternal"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;

        let adapter = make_adapter(&server);
        let user = ChannelUser {
            platform_id: "C123/111.222".into(),
            display_name: None,
        };
        let content = ChannelContent::FileData {
            data: b"fake-image-bytes".to_vec(),
            filename: "output.png".into(),
            mime_type: "image/png".into(),
        };

        adapter.send(&user, content).await.unwrap();
        // wiremock asserts all mocks were hit
    }

    #[tokio::test]
    async fn send_in_thread_file_data_calls_upload_file() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/files.getUploadURLExternal"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": true,
                "upload_url": format!("{}/upload-target", server.uri()),
                "file_id": "F456"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/upload-target"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/files.completeUploadExternal"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .mount(&server)
            .await;

        let adapter = make_adapter(&server);
        let user = ChannelUser {
            platform_id: "C123/111.222".into(),
            display_name: None,
        };
        let content = ChannelContent::FileData {
            data: b"fake-image-bytes".to_vec(),
            filename: "output.png".into(),
            mime_type: "image/png".into(),
        };

        adapter
            .send_in_thread(&user, content, "111.222")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn extract_image_skips_non_image_files() {
        let server = MockServer::start().await;
        let client =
            SlackApiClient::with_base_url("xoxb-test".into(), "xapp-test".into(), server.uri())
                .unwrap();

        let event = serde_json::json!({
            "files": [{
                "mimetype": "application/pdf",
                "name": "doc.pdf",
                "url_private_download": format!("{}/files/doc.pdf", server.uri())
            }]
        });

        let result = extract_image_from_event(&event, &client).await;
        assert!(result.is_none(), "should skip non-image files");
    }

    // -- should_process tests -------------------------------------------------

    #[test]
    fn mention_mode_accepts_dm() {
        let result = should_process(
            &SlackListenMode::Mention,
            "U_BOT",
            "D123ABC", // DM channel
            "hello",
            None,
            "111.222",
            false,
        );
        assert_eq!(
            result,
            ShouldProcessResult::Accept,
            "DMs should always be accepted in Mention mode"
        );
    }

    #[test]
    fn mention_mode_accepts_at_mention() {
        let result = should_process(
            &SlackListenMode::Mention,
            "U_BOT",
            "C123",
            "hey <@U_BOT> help me",
            None,
            "111.222",
            false,
        );
        assert_eq!(
            result,
            ShouldProcessResult::AcceptAndTrack,
            "@-mention should accept and track thread"
        );
    }

    #[test]
    fn mention_mode_accepts_tracked_thread_reply() {
        let result = should_process(
            &SlackListenMode::Mention,
            "U_BOT",
            "C123",
            "thanks!",
            Some("100.000"), // thread_ts differs from ts → is a reply
            "111.222",
            true, // thread is tracked
        );
        assert_eq!(
            result,
            ShouldProcessResult::Accept,
            "replies in tracked threads should be accepted"
        );
    }

    #[test]
    fn mention_mode_rejects_unmentioned_channel_message() {
        let result = should_process(
            &SlackListenMode::Mention,
            "U_BOT",
            "C123",
            "just chatting",
            None,
            "111.222",
            false,
        );
        assert_eq!(
            result,
            ShouldProcessResult::Reject,
            "channel messages without mention should be rejected"
        );
    }

    #[test]
    fn mention_mode_rejects_untracked_thread_reply() {
        let result = should_process(
            &SlackListenMode::Mention,
            "U_BOT",
            "C123",
            "reply without prior mention",
            Some("100.000"),
            "111.222",
            false, // thread is NOT tracked
        );
        assert_eq!(
            result,
            ShouldProcessResult::Reject,
            "replies in untracked threads should be rejected"
        );
    }

    #[test]
    fn all_mode_accepts_everything() {
        let result = should_process(
            &SlackListenMode::All,
            "U_BOT",
            "C123",
            "random message",
            None,
            "111.222",
            false,
        );
        assert_eq!(
            result,
            ShouldProcessResult::Accept,
            "All mode should accept every message"
        );
    }
}
