//! Mattermost `ChannelAdapter` implementation.
//!
//! Connects to the Mattermost WebSocket API, authenticates with a token
//! challenge, and yields inbound `posted` events as [`ChannelMessage`]s.
//! Automatic exponential-backoff reconnection is built in.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelUser, ToolHandler,
    types::conversation::ChannelType,
};
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest, is_audio_mime};
use async_trait::async_trait;
use chrono::Utc;
use futures::SinkExt;
use futures::stream::Stream;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::mattermost::client::MattermostClient;

use crate::common::{BACKOFF_MIN, sleep_backoff};

/// Maximum audio download size (25 MB).
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

/// Mattermost `ChannelAdapter`. Connects via WebSocket and posts via REST.
pub struct MattermostAdapter {
    client: Arc<MattermostClient>,
    allowed_channels: Vec<String>,
    allowed_users: Vec<String>,
    /// Resolved bot user ID (set after a successful `get_me()` call in `start()`).
    bot_user_id: Arc<tokio::sync::RwLock<Option<String>>>,
    /// Stores the post ID that the hourglass reaction was added to in `on_message_received`,
    /// keyed by platform_id (channel_id/thread_root).  Used for precise removal in
    /// `on_turn_start` regardless of threading.
    pending_post_ids: Arc<Mutex<HashMap<String, String>>>,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    /// Optional audio transcription provider for voice messages.
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    /// BCP-47 language hint passed to the transcription provider.
    transcription_language: Option<String>,
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
            pending_post_ids: Arc::new(Mutex::new(HashMap::new())),
            stop_tx,
            stop_rx,
            transcription: None,
            transcription_language: None,
        }
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

    #[allow(dead_code)]
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
        let transcription = self.transcription.clone();
        let transcription_language = self.transcription_language.clone();

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

                                    if let Some(mut msg) = parse_posted_event(
                                        &payload,
                                        &bot_user_id,
                                        &allowed_channels,
                                        &allowed_users,
                                    ) {
                                        // Check for audio file attachments requiring transcription.
                                        if let Some(transcript) = transcribe_mattermost_files(
                                            &payload,
                                            &client,
                                            &transcription,
                                            &transcription_language,
                                        )
                                        .await
                                        {
                                            // Prepend transcript to message text.
                                            let existing = match &msg.content {
                                                ChannelContent::Text(t) if !t.is_empty() => {
                                                    Some(t.clone())
                                                }
                                                _ => None,
                                            };
                                            let text = match existing {
                                                Some(t) => format!("{transcript}\n{t}"),
                                                None => transcript,
                                            };
                                            msg.content = ChannelContent::Text(text);
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
        let (channel_id, root_id) = parse_platform_id(&user.platform_id);
        match content {
            ChannelContent::Text(text) => {
                self.client
                    .create_post(&channel_id, &text, root_id.as_deref())
                    .await?;
            }
            ChannelContent::FileData {
                data,
                filename,
                mime_type: _,
            } => {
                let file_ids = self
                    .client
                    .upload_file(&channel_id, &filename, data)
                    .await?;
                if !file_ids.is_empty() {
                    self.client
                        .create_post_with_files(&channel_id, "", root_id.as_deref(), file_ids)
                        .await?;
                }
            }
            _ => {}
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
        match content {
            ChannelContent::Text(text) => {
                self.client
                    .create_post(&channel_id, &text, Some(thread_id))
                    .await?;
            }
            ChannelContent::FileData {
                data,
                filename,
                mime_type: _,
            } => {
                let file_ids = self
                    .client
                    .upload_file(&channel_id, &filename, data)
                    .await?;
                if !file_ids.is_empty() {
                    self.client
                        .create_post_with_files(&channel_id, "", Some(thread_id), file_ids)
                        .await?;
                }
            }
            _ => {}
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
        super::tools::build_mattermost_tools(
            channel_id,
            post_id,
            thread_id,
            bot_user_id,
            self.client.clone(),
        )
    }

    /// Add ⏳ hourglass reaction immediately on message receipt (before the conv lock).
    /// Stores the reacted post_id keyed by platform_id for precise removal in `on_turn_start`.
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
        } else {
            self.pending_post_ids
                .lock()
                .await
                .insert(msg.sender.platform_id.clone(), post_id);
        }
        Ok(())
    }

    /// Remove ⏳ (using the stored post_id) and send typing event when a turn starts.
    async fn on_turn_start(&self, user: &ChannelUser, _conv_id: Uuid) -> Result<()> {
        let (channel_id, _) = parse_platform_id(&user.platform_id);
        let bot_user_id = self
            .bot_user_id
            .try_read()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default();
        if !bot_user_id.is_empty() {
            let post_id = self.pending_post_ids.lock().await.remove(&user.platform_id);
            if let Some(pid) = post_id
                && let Err(e) = self
                    .client
                    .remove_reaction(&bot_user_id, &pid, "hourglass_flowing_sand")
                    .await
            {
                debug!(error = %e, "mattermost: failed to remove hourglass reaction (best-effort)");
            }
        }
        if let Err(e) = self.client.send_typing(&channel_id).await {
            debug!(error = %e, "mattermost: failed to send typing indicator (best-effort)");
        }
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

    // Check for file attachments (voice messages may have no text).
    let has_file_ids = post
        .get("file_ids")
        .and_then(|v| v.as_array())
        .is_some_and(|a| !a.is_empty());

    if text.is_empty() && !has_file_ids {
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

/// Extract `file_ids` from the nested post JSON inside a `posted` WebSocket event.
fn extract_file_ids(payload: &serde_json::Value) -> Vec<String> {
    let post_str = match payload.pointer("/data/post").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return vec![],
    };
    let post: serde_json::Value = match serde_json::from_str(post_str) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    post.get("file_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Check for audio file attachments in a posted event and transcribe them.
///
/// Returns a formatted `[Voice message]: ...` string if any audio was
/// successfully transcribed, or `None` otherwise.
async fn transcribe_mattermost_files(
    payload: &serde_json::Value,
    client: &MattermostClient,
    transcription: &Option<Arc<dyn TranscriptionProvider>>,
    language: &Option<String>,
) -> Option<String> {
    let file_ids = extract_file_ids(payload);
    if file_ids.is_empty() {
        return None;
    }

    let provider = match transcription {
        Some(p) => p,
        None => {
            debug!("mattermost: post has file_ids but no transcription provider configured");
            return None;
        }
    };

    let mut transcripts = Vec::new();
    for file_id in &file_ids {
        // Fetch file metadata to check MIME type.
        let info = match client.get_file_info(file_id).await {
            Ok(i) => i,
            Err(e) => {
                warn!(error = %e, file_id, "mattermost: failed to fetch file info");
                continue;
            }
        };

        if !is_audio_mime(&info.mime_type) {
            debug!(
                file_id,
                mime_type = %info.mime_type,
                "mattermost: skipping non-audio attachment"
            );
            continue;
        }

        // Download the file bytes.
        let data = match client.download_file(file_id).await {
            Ok(d) => d,
            Err(e) => {
                warn!(error = %e, file_id, "mattermost: failed to download audio file");
                continue;
            }
        };

        if data.len() > MAX_AUDIO_BYTES {
            warn!(
                size = data.len(),
                limit = MAX_AUDIO_BYTES,
                file_id,
                "mattermost: audio file too large; skipping"
            );
            continue;
        }

        let request = TranscriptionRequest {
            audio_data: data,
            mime_type: info.mime_type.clone(),
            filename: Some(info.name.clone()),
            language: language.clone(),
        };
        match provider.transcribe(request).await {
            Ok(result) => transcripts.push(result.text),
            Err(e) => {
                warn!(error = %e, file_id, "mattermost: audio transcription failed");
            }
        }
    }

    if transcripts.is_empty() {
        return None;
    }

    Some(format!("[Voice message]: {}", transcripts.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_transcription_sets_provider() {
        use assistant_transcription::{
            TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
        };

        #[derive(Debug)]
        struct DummyProvider;

        #[async_trait]
        impl TranscriptionProvider for DummyProvider {
            fn name(&self) -> &str {
                "dummy"
            }

            async fn transcribe(&self, _req: TranscriptionRequest) -> Result<TranscriptionResult> {
                Ok(TranscriptionResult {
                    text: "hello".to_string(),
                    language: None,
                    duration_secs: None,
                })
            }
        }

        let client = Arc::new(
            crate::mattermost::client::MattermostClient::new("http://localhost", "tok").unwrap(),
        );
        let adapter = MattermostAdapter::new(client, vec![], vec![]);
        assert!(adapter.transcription.is_none());

        let adapter = adapter.with_transcription(Arc::new(DummyProvider), Some("en".to_string()));
        assert!(adapter.transcription.is_some());
        assert_eq!(adapter.transcription_language.as_deref(), Some("en"));
    }

    #[test]
    fn parse_posted_event_text_still_works() {
        let payload = serde_json::json!({
            "event": "posted",
            "broadcast": { "channel_id": "ch1" },
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "user_id": "u1",
                    "message": "hello world",
                    "root_id": ""
                }).to_string()
            }
        });
        let msg = parse_posted_event(&payload, "bot", &[], &[]);
        assert!(msg.is_some());
        let msg = msg.unwrap();
        match &msg.content {
            ChannelContent::Text(t) => assert_eq!(t, "hello world"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn parse_posted_event_empty_text_no_files_returns_none() {
        let payload = serde_json::json!({
            "event": "posted",
            "broadcast": { "channel_id": "ch1" },
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "user_id": "u1",
                    "message": "",
                    "root_id": ""
                }).to_string()
            }
        });
        assert!(parse_posted_event(&payload, "bot", &[], &[]).is_none());
    }

    #[test]
    fn parse_posted_event_empty_text_with_file_ids_returns_some() {
        let payload = serde_json::json!({
            "event": "posted",
            "broadcast": { "channel_id": "ch1" },
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "user_id": "u1",
                    "message": "",
                    "root_id": "",
                    "file_ids": ["f1"]
                }).to_string()
            }
        });
        let msg = parse_posted_event(&payload, "bot", &[], &[]);
        assert!(
            msg.is_some(),
            "should accept posts with file_ids even if text is empty"
        );
    }

    #[test]
    fn extract_file_ids_parses_correctly() {
        let payload = serde_json::json!({
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "file_ids": ["f1", "f2"]
                }).to_string()
            }
        });
        let ids = extract_file_ids(&payload);
        assert_eq!(ids, vec!["f1".to_string(), "f2".to_string()]);
    }

    #[test]
    fn extract_file_ids_empty_when_none() {
        let payload = serde_json::json!({
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "message": "hi"
                }).to_string()
            }
        });
        let ids = extract_file_ids(&payload);
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn transcribe_mattermost_files_no_provider_returns_none() {
        let payload = serde_json::json!({
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "file_ids": ["f1"]
                }).to_string()
            }
        });
        let client =
            crate::mattermost::client::MattermostClient::new("http://localhost", "tok").unwrap();
        let result = transcribe_mattermost_files(&payload, &client, &None, &None).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn transcribe_mattermost_files_no_file_ids_returns_none() {
        let payload = serde_json::json!({
            "data": {
                "post": serde_json::json!({
                    "id": "p1",
                    "message": "text only"
                }).to_string()
            }
        });
        let client =
            crate::mattermost::client::MattermostClient::new("http://localhost", "tok").unwrap();
        let result = transcribe_mattermost_files(&payload, &client, &None, &None).await;
        assert!(result.is_none());
    }
}
