//! Matrix `ChannelAdapter` implementation.
//!
//! Drives a long-poll `/sync` loop, yielding inbound `m.room.message` events
//! as [`ChannelMessage`]s.  The `next_batch` token is persisted to disk so
//! the bot does not replay old messages after a restart.
//!
//! Auto-accepts room invitations via `POST /join/<room_id>`.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser, ToolHandler,
};
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream::Stream;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::client::MatrixClient;

/// Maximum audio file size downloaded for transcription (25 MB — Whisper API limit).
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

/// Maximum image file size downloaded (10 MB).
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// Matrix `ChannelAdapter`.
pub struct MatrixAdapter {
    client: Arc<MatrixClient>,
    allowed_rooms: Vec<String>,
    allowed_users: Vec<String>,
    /// Path to persist the `next_batch` token.
    next_batch_path: PathBuf,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    /// Optional audio transcription provider for voice messages.
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    /// BCP-47 language hint passed to the transcription provider.
    transcription_language: Option<String>,
    /// Stores the reaction event IDs for pending ⏳ hourglass reactions,
    /// keyed by room_id.  A VecDeque is used so that multiple queued messages
    /// in the same room are redacted in FIFO order without losing track.
    pending_reactions: Arc<Mutex<HashMap<String, VecDeque<String>>>>,
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
            transcription: None,
            transcription_language: None,
            pending_reactions: Arc::new(Mutex::new(HashMap::new())),
        }
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
        let transcription = self.transcription.clone();
        let transcription_language = self.transcription_language.clone();

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

                                match msgtype {
                                    "m.text" => {
                                        let text = event
                                            .content
                                            .get("body")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        if text.is_empty() {
                                            continue;
                                        }
                                        if event.sender == client.user_id {
                                            debug!(sender = %event.sender, "Matrix: ignoring self-message");
                                            continue;
                                        }
                                        if !allowed_rooms.is_empty()
                                            && !allowed_rooms.contains(room_id)
                                        {
                                            debug!(
                                                room_id,
                                                "Matrix: room not in allowlist; dropping"
                                            );
                                            continue;
                                        }
                                        if !allowed_users.is_empty()
                                            && !allowed_users.contains(&event.sender)
                                        {
                                            debug!(sender = %event.sender, "Matrix: user not in allowlist; dropping");
                                            continue;
                                        }
                                        let msg = build_message(
                                            room_id,
                                            &event.sender,
                                            event.event_id.clone(),
                                            ChannelContent::Text(text),
                                        );
                                        if tx.send(msg).await.is_err() {
                                            return;
                                        }
                                    }

                                    "m.audio" => {
                                        if event.sender == client.user_id {
                                            debug!(sender = %event.sender, "Matrix: ignoring self voice message");
                                            continue;
                                        }
                                        if !allowed_rooms.is_empty()
                                            && !allowed_rooms.contains(room_id)
                                        {
                                            debug!(
                                                room_id,
                                                "Matrix: room not in allowlist; dropping voice"
                                            );
                                            continue;
                                        }
                                        if !allowed_users.is_empty()
                                            && !allowed_users.contains(&event.sender)
                                        {
                                            debug!(sender = %event.sender, "Matrix: user not in allowlist; dropping voice");
                                            continue;
                                        }
                                        let Some(ref provider) = transcription else {
                                            warn!(
                                                room_id,
                                                sender = %event.sender,
                                                "Matrix: received voice message but no transcription provider configured; dropping"
                                            );
                                            continue;
                                        };
                                        let mxc_url =
                                            match event.content.get("url").and_then(|v| v.as_str())
                                            {
                                                Some(u) => u.to_string(),
                                                None => {
                                                    warn!(
                                                        room_id,
                                                        "Matrix: m.audio event missing url field"
                                                    );
                                                    continue;
                                                }
                                            };
                                        let (audio_bytes, mime_type) = match client
                                            .download_media(&mxc_url, MAX_AUDIO_BYTES)
                                            .await
                                        {
                                            Ok(v) => v,
                                            Err(e) => {
                                                warn!(error = %e, room_id, "Matrix: failed to download audio");
                                                continue;
                                            }
                                        };
                                        let request = TranscriptionRequest {
                                            audio_data: audio_bytes,
                                            mime_type,
                                            filename: None,
                                            language: transcription_language.clone(),
                                        };
                                        let transcript = match provider.transcribe(request).await {
                                            Ok(r) => r.text,
                                            Err(e) => {
                                                warn!(error = %e, room_id, "Matrix: audio transcription failed");
                                                continue;
                                            }
                                        };
                                        let text = format!("[Voice message]: {transcript}");
                                        let msg = build_message(
                                            room_id,
                                            &event.sender,
                                            event.event_id.clone(),
                                            ChannelContent::Text(text),
                                        );
                                        if tx.send(msg).await.is_err() {
                                            return;
                                        }
                                    }

                                    "m.image" => {
                                        if event.sender == client.user_id {
                                            debug!(sender = %event.sender, "Matrix: ignoring self image");
                                            continue;
                                        }
                                        if !allowed_rooms.is_empty()
                                            && !allowed_rooms.contains(room_id)
                                        {
                                            debug!(
                                                room_id,
                                                "Matrix: room not in allowlist; dropping image"
                                            );
                                            continue;
                                        }
                                        if !allowed_users.is_empty()
                                            && !allowed_users.contains(&event.sender)
                                        {
                                            debug!(sender = %event.sender, "Matrix: user not in allowlist; dropping image");
                                            continue;
                                        }
                                        let mxc_url =
                                            match event.content.get("url").and_then(|v| v.as_str())
                                            {
                                                Some(u) => u.to_string(),
                                                None => {
                                                    warn!(
                                                        room_id,
                                                        "Matrix: m.image event missing url field"
                                                    );
                                                    continue;
                                                }
                                            };
                                        let filename = event
                                            .content
                                            .get("body")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("image")
                                            .to_string();
                                        let (image_bytes, mime_type) = match client
                                            .download_media(&mxc_url, MAX_IMAGE_BYTES)
                                            .await
                                        {
                                            Ok(v) => v,
                                            Err(e) => {
                                                warn!(error = %e, room_id, "Matrix: failed to download image");
                                                continue;
                                            }
                                        };
                                        let msg = build_message(
                                            room_id,
                                            &event.sender,
                                            event.event_id.clone(),
                                            ChannelContent::FileData {
                                                data: image_bytes,
                                                filename,
                                                mime_type,
                                            },
                                        );
                                        if tx.send(msg).await.is_err() {
                                            return;
                                        }
                                    }

                                    _ => {} // ignore other message types
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
        let room_id = &user.platform_id;
        match content {
            ChannelContent::Text(text) => {
                self.client.send_message(room_id, &text).await?;
            }
            ChannelContent::FileData {
                data,
                filename,
                mime_type,
            } => {
                let size = data.len();
                let mxc_uri = self
                    .client
                    .upload_media(data, &filename, &mime_type)
                    .await?;

                // Route audio files to send_audio, images to send_image
                if is_audio_mime_type(&mime_type) {
                    self.client
                        .send_audio(room_id, &mxc_uri, &filename, &mime_type, size, None)
                        .await?;
                } else {
                    self.client
                        .send_image(room_id, &mxc_uri, &filename, &mime_type, size)
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

    /// Add ⏳ hourglass reaction immediately on message receipt (before the conv lock).
    /// Spawns a detached task to avoid blocking the inbound stream on homeserver I/O.
    /// Reaction event IDs are stored in a per-room FIFO queue for later redaction.
    async fn on_message_received(&self, msg: &ChannelMessage) -> Result<()> {
        let room_id = msg
            .metadata
            .get("room_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&msg.sender.platform_id)
            .to_string();
        let Some(event_id) = msg.platform_message_id.clone() else {
            return Ok(());
        };
        let client = self.client.clone();
        let pending = self.pending_reactions.clone();
        tokio::spawn(async move {
            match client.add_reaction(&room_id, &event_id, "⏳").await {
                Ok(reaction_id) => {
                    pending
                        .lock()
                        .await
                        .entry(room_id)
                        .or_default()
                        .push_back(reaction_id);
                }
                Err(e) => {
                    debug!(error = %e, "matrix: failed to add hourglass reaction (best-effort)");
                }
            }
        });
        Ok(())
    }

    /// Redact ⏳ (FIFO) and send typing indicator when a turn starts.
    async fn on_turn_start(&self, user: &ChannelUser, _conv_id: Uuid) -> Result<()> {
        let room_id = &user.platform_id;
        let reaction_id = self
            .pending_reactions
            .lock()
            .await
            .get_mut(room_id)
            .and_then(|q| q.pop_front());
        if let Some(rid) = reaction_id {
            if let Err(e) = self.client.redact_event(room_id, &rid).await {
                debug!(error = %e, "matrix: failed to redact hourglass reaction (best-effort)");
            }
        }
        if let Err(e) = self.client.send_typing(room_id, true).await {
            debug!(error = %e, "matrix: failed to send typing indicator (best-effort)");
        }
        Ok(())
    }

    /// Clear typing indicator on successful turn.
    async fn on_turn_success(
        &self,
        user: &ChannelUser,
        _answer: &str,
        _attachments: &[assistant_core::Attachment],
    ) -> Result<()> {
        if let Err(e) = self.client.send_typing(&user.platform_id, false).await {
            debug!(error = %e, "matrix: failed to clear typing indicator (best-effort)");
        }
        Ok(())
    }

    async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()> {
        let room_id = &user.platform_id;
        if let Err(e) = self.client.send_typing(room_id, false).await {
            debug!(error = %e, "matrix: failed to clear typing indicator (best-effort)");
        }
        let _ = self
            .client
            .send_message(room_id, &format!("Sorry, I encountered an error: {err}"))
            .await;
        Ok(())
    }
}

/// Check if a MIME type represents an audio file.
fn is_audio_mime_type(mime_type: &str) -> bool {
    matches!(
        mime_type,
        "audio/ogg"
            | "audio/mpeg"
            | "audio/wav"
            | "audio/webm"
            | "audio/mp4"
            | "audio/aac"
            | "audio/flac"
            | "audio/x-wav"
            | "audio/opus"
    ) || mime_type.starts_with("audio/")
}

fn build_message(
    room_id: &str,
    sender: &str,
    event_id: Option<String>,
    content: ChannelContent,
) -> ChannelMessage {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "room_id".to_string(),
        serde_json::Value::String(room_id.to_string()),
    );
    metadata.insert(
        "sender".to_string(),
        serde_json::Value::String(sender.to_string()),
    );
    ChannelMessage {
        channel_type: ChannelType::Matrix,
        platform_message_id: event_id,
        sender: ChannelUser {
            platform_id: room_id.to_string(),
            display_name: Some(sender.to_string()),
        },
        content,
        thread_id: None,
        timestamp: Utc::now(),
        metadata,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assistant_transcription::{
        TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
    };
    use async_trait::async_trait;
    use futures::StreamExt;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::client::MatrixClient;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_client(server: &MockServer) -> Arc<MatrixClient> {
        Arc::new(
            MatrixClient::new_with_token(server.uri(), "test-token", "@bot:example.com").unwrap(),
        )
    }

    fn make_adapter(client: Arc<MatrixClient>) -> MatrixAdapter {
        MatrixAdapter::new(client, vec![], vec![])
    }

    /// Build a minimal sync JSON with one message event in room "!room:example.com".
    fn sync_with_event(
        msgtype: &str,
        sender: &str,
        extra_content: serde_json::Value,
    ) -> serde_json::Value {
        let mut content = serde_json::json!({ "msgtype": msgtype });
        if let serde_json::Value::Object(ref mut map) = content {
            if let serde_json::Value::Object(extra) = extra_content {
                map.extend(extra);
            }
        }
        serde_json::json!({
            "next_batch": "s1",
            "rooms": {
                "join": {
                    "!room:example.com": {
                        "timeline": {
                            "events": [{
                                "type": "m.room.message",
                                "sender": sender,
                                "event_id": "$evt1",
                                "content": content
                            }]
                        }
                    }
                }
            }
        })
    }

    /// Mount a two-sync sequence: first an empty bootstrap, then the real event.
    async fn mount_two_syncs(server: &MockServer, event_sync: serde_json::Value) {
        let bootstrap = serde_json::json!({ "next_batch": "s0", "rooms": {} });
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(bootstrap)
                    .append_header("X-Seq", "0"),
            )
            .up_to_n_times(1)
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(event_sync))
            .mount(server)
            .await;
    }

    /// A transcription provider that always returns a fixed transcript.
    struct FakeTranscriber(String);

    #[async_trait]
    impl TranscriptionProvider for FakeTranscriber {
        fn name(&self) -> &str {
            "fake"
        }
        async fn transcribe(
            &self,
            _req: TranscriptionRequest,
        ) -> anyhow::Result<TranscriptionResult> {
            Ok(TranscriptionResult {
                text: self.0.clone(),
                language: None,
                duration_secs: None,
            })
        }
    }

    // ── voice tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn voice_message_transcribed_and_emitted() {
        let server = MockServer::start().await;
        // Media download mock
        Mock::given(method("GET"))
            .and(path("/_matrix/media/v3/download/example.com/audio1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"audio".as_ref())
                    .append_header("Content-Type", "audio/ogg"),
            )
            .mount(&server)
            .await;
        let event_sync = sync_with_event(
            "m.audio",
            "@user:example.com",
            serde_json::json!({ "url": "mxc://example.com/audio1", "body": "voice.ogg" }),
        );
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let provider = Arc::new(FakeTranscriber("hello world".to_string()));
        let adapter = make_adapter(client).with_transcription(provider, None);

        let mut stream = adapter.start().await.unwrap();
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await
            .expect("timeout")
            .expect("stream ended");

        assert!(
            matches!(&msg.content, ChannelContent::Text(t) if t == "[Voice message]: hello world"),
            "unexpected content: {:?}",
            msg.content
        );
    }

    #[tokio::test]
    async fn voice_message_no_provider_dropped() {
        let server = MockServer::start().await;
        let event_sync = sync_with_event(
            "m.audio",
            "@user:example.com",
            serde_json::json!({ "url": "mxc://example.com/audio2" }),
        );
        // Third sync returns empty so the stream doesn't hang.
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "next_batch": "s2", "rooms": {} })),
            )
            .mount(&server)
            .await;
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let adapter = make_adapter(client); // no transcription provider

        let mut stream = adapter.start().await.unwrap();
        // Expect NO message within a short window.
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(200), stream.next()).await;
        assert!(
            result.is_err(),
            "expected timeout — no message should be emitted"
        );
    }

    #[tokio::test]
    async fn voice_message_from_self_dropped() {
        let server = MockServer::start().await;
        let event_sync = sync_with_event(
            "m.audio",
            "@bot:example.com", // same as client user_id
            serde_json::json!({ "url": "mxc://example.com/audio3" }),
        );
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "next_batch": "s2", "rooms": {} })),
            )
            .mount(&server)
            .await;
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let provider = Arc::new(FakeTranscriber("ignored".to_string()));
        let adapter = make_adapter(client).with_transcription(provider, None);

        let mut stream = adapter.start().await.unwrap();
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(200), stream.next()).await;
        assert!(result.is_err(), "self voice message should be dropped");
    }

    #[tokio::test]
    async fn voice_message_non_allowed_user_dropped() {
        let server = MockServer::start().await;
        let event_sync = sync_with_event(
            "m.audio",
            "@stranger:example.com",
            serde_json::json!({ "url": "mxc://example.com/audio4" }),
        );
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "next_batch": "s2", "rooms": {} })),
            )
            .mount(&server)
            .await;
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let provider = Arc::new(FakeTranscriber("ignored".to_string()));
        // Only allow "@allowed:example.com"
        let mut adapter =
            MatrixAdapter::new(client, vec![], vec!["@allowed:example.com".to_string()]);
        adapter = adapter.with_transcription(provider, None);

        let mut stream = adapter.start().await.unwrap();
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(200), stream.next()).await;
        assert!(
            result.is_err(),
            "non-allowed user voice message should be dropped"
        );
    }

    // ── image tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn image_message_emitted_as_file_data() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/media/v3/download/example.com/img1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"imgdata".as_ref())
                    .append_header("Content-Type", "image/jpeg"),
            )
            .mount(&server)
            .await;
        let event_sync = sync_with_event(
            "m.image",
            "@user:example.com",
            serde_json::json!({ "url": "mxc://example.com/img1", "body": "photo.jpg" }),
        );
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let adapter = make_adapter(client);

        let mut stream = adapter.start().await.unwrap();
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await
            .expect("timeout")
            .expect("stream ended");

        match &msg.content {
            ChannelContent::FileData {
                data,
                filename,
                mime_type,
            } => {
                assert_eq!(data, b"imgdata");
                assert_eq!(filename, "photo.jpg");
                assert_eq!(mime_type, "image/jpeg");
            }
            other => panic!("expected FileData, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn image_message_from_self_dropped() {
        let server = MockServer::start().await;
        let event_sync = sync_with_event(
            "m.image",
            "@bot:example.com",
            serde_json::json!({ "url": "mxc://example.com/img2", "body": "photo.jpg" }),
        );
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "next_batch": "s2", "rooms": {} })),
            )
            .mount(&server)
            .await;
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let adapter = make_adapter(client);

        let mut stream = adapter.start().await.unwrap();
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(200), stream.next()).await;
        assert!(result.is_err(), "self image should be dropped");
    }

    #[tokio::test]
    async fn image_message_non_allowed_user_dropped() {
        let server = MockServer::start().await;
        let event_sync = sync_with_event(
            "m.image",
            "@stranger:example.com",
            serde_json::json!({ "url": "mxc://example.com/img3", "body": "photo.jpg" }),
        );
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "next_batch": "s2", "rooms": {} })),
            )
            .mount(&server)
            .await;
        mount_two_syncs(&server, event_sync).await;

        let client = make_client(&server);
        let adapter = MatrixAdapter::new(client, vec![], vec!["@allowed:example.com".to_string()]);

        let mut stream = adapter.start().await.unwrap();
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(200), stream.next()).await;
        assert!(result.is_err(), "non-allowed user image should be dropped");
    }

    // ── outbound attachment tests ────────────────────────────────────────────

    #[tokio::test]
    async fn send_file_data_uploads_and_sends_image() {
        let server = MockServer::start().await;

        // Media upload
        Mock::given(method("POST"))
            .and(wiremock::matchers::path_regex(r"/_matrix/media/v3/upload"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({ "content_uri": "mxc://example.com/outbound1" }),
                ),
            )
            .mount(&server)
            .await;

        // m.image event send
        Mock::given(method("PUT"))
            .and(wiremock::matchers::path_regex(
                r"/_matrix/client/v3/rooms/.+/send/m.room.message/.+",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "event_id": "$img_out" })),
            )
            .mount(&server)
            .await;

        let client = make_client(&server);
        let adapter = make_adapter(client);
        let user = assistant_core::ChannelUser {
            platform_id: "!room:example.com".into(),
            display_name: None,
        };

        let content = ChannelContent::FileData {
            data: b"fake-png-bytes".to_vec(),
            filename: "result.png".into(),
            mime_type: "image/png".into(),
        };

        adapter.send(&user, content).await.unwrap();
        // wiremock asserts mocks were hit
    }
}
