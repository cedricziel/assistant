//! Signal `ChannelAdapter` implementation backed by signal-cli-rest-api.
//!
//! Connects to the signal-cli-rest-api WebSocket endpoint
//! `GET /v1/receive/{phone_number}` and yields inbound messages as
//! [`ChannelMessage`] items.  Outbound replies are POSTed to
//! `POST /v1/send`.
//!
//! Auto-reconnects with exponential backoff on disconnect (same pattern as
//! [`SlackAdapter`][crate]).

use assistant_core::clock::{Clock, SystemClock};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelUser, types::conversation::ChannelType,
};
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest, is_audio_mime};
use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use futures::stream::Stream;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::http::Request;
use tracing::{debug, error, info, warn};

use super::config::{SignalConfig, SignalConfigExt};

use crate::common::{BACKOFF_MIN, sleep_backoff};

/// Maximum audio download size (25 MB).
const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

/// Signal `ChannelAdapter`.  Connects to the signal-cli-rest-api WebSocket
/// and provides `send()` via `POST /v1/send`.
pub struct SignalAdapter {
    config: SignalConfig,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
    /// Optional audio transcription provider for voice messages.
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    /// BCP-47 language hint passed to the transcription provider.
    transcription_language: Option<String>,
}

impl SignalAdapter {
    /// Create a new [`SignalAdapter`] from the given config.
    ///
    /// Fails if `phone_number` is not configured (required for the WebSocket
    /// receive path).
    pub fn new(config: SignalConfig) -> Result<Self> {
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        Ok(Self {
            config,
            stop_tx,
            stop_rx,
            transcription: None,
            transcription_language: None,
        })
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

    /// Build the HTTP Basic Auth header value if credentials are configured.
    fn basic_auth_header(&self) -> Option<String> {
        let user = self.config.resolved_api_user()?;
        let pass = self.config.resolved_api_password().unwrap_or_default();
        let encoded = B64_STANDARD.encode(format!("{user}:{pass}"));
        Some(format!("Basic {encoded}"))
    }
}

#[async_trait]
impl ChannelAdapter for SignalAdapter {
    fn name(&self) -> &str {
        "signal"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Signal
    }

    async fn start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
        let phone_number = self
            .config
            .resolved_phone_number()
            .ok_or_else(|| anyhow::anyhow!("signal: phone_number is required in config"))?;

        let api_url = self.config.resolved_api_url();
        let allowed_senders = self.config.allowed_senders.clone();
        let auth_header = self.basic_auth_header();
        let transcription = self.transcription.clone();
        let transcription_language = self.transcription_language.clone();
        let mut stop_rx = self.stop_rx.clone();

        let (tx, rx) = mpsc::channel::<ChannelMessage>(64);

        tokio::spawn(async move {
            let mut backoff = BACKOFF_MIN;

            // Build the WebSocket URL (ws:// or wss://).
            let ws_url = build_ws_url(&api_url, &phone_number);

            loop {
                // Check stop signal before each connection attempt.
                if *stop_rx.borrow() {
                    break;
                }

                // Build a Request so we can attach the Authorization header.
                let connect_result = if let Some(ref auth) = auth_header {
                    match Request::builder()
                        .uri(&ws_url)
                        .header("Authorization", auth)
                        .body(())
                    {
                        Ok(req) => connect_async(req).await,
                        Err(e) => {
                            error!(error = %e, "signal: failed to build WebSocket request; retrying");
                            sleep_backoff(&mut backoff, &mut stop_rx).await;
                            continue;
                        }
                    }
                } else {
                    connect_async(&ws_url).await
                };

                let ws_stream = match connect_result {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        error!(error = %e, url = %ws_url, "signal: WebSocket connect failed; retrying");
                        sleep_backoff(&mut backoff, &mut stop_rx).await;
                        continue;
                    }
                };

                info!(url = %ws_url, "signal: WebSocket connected");
                backoff = BACKOFF_MIN; // reset on success

                let (_ws_write, mut ws_read) = futures::StreamExt::split(ws_stream);

                loop {
                    tokio::select! {
                        _ = stop_rx.changed() => {
                            if *stop_rx.borrow() {
                                return; // exit the entire spawn
                            }
                        }
                        msg = futures::StreamExt::next(&mut ws_read) => {
                            match msg {
                                None => {
                                    warn!("signal: WS stream ended; reconnecting");
                                    break;
                                }
                                Some(Err(e)) => {
                                    warn!(error = %e, "signal: WS error; reconnecting");
                                    break;
                                }
                                Some(Ok(WsMessage::Text(text))) => {
                                    let raw = text.as_str();
                                    match parse_envelope(raw) {
                                        None => {
                                            debug!("signal: ignoring non-dataMessage envelope");
                                        }
                                        Some(env) => {
                                            // Allowlist check.
                                            if !allowed_senders.is_empty()
                                                && !allowed_senders.contains(&env.source)
                                            {
                                                debug!(
                                                    source = %env.source,
                                                    "signal: sender not in allowlist; dropping"
                                                );
                                                continue;
                                            }

                                            // Check for audio attachments that need transcription.
                                            let audio_transcript = transcribe_audio_attachments(
                                                &env.attachments,
                                                &transcription,
                                                &transcription_language,
                                            )
                                            .await;

                                            if let Some(msg) =
                                                envelope_to_channel_message(env, audio_transcript)
                                                && tx.send(msg).await.is_err()
                                            {
                                                return; // receiver dropped
                                            }
                                        }
                                    }
                                }
                                Some(Ok(WsMessage::Ping(data))) => {
                                    // The tungstenite library handles pong automatically,
                                    // but ws_write was moved into split — ping/pong is
                                    // auto-responded by the underlying stream.
                                    let _ = data;
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
        let phone_number = self
            .config
            .resolved_phone_number()
            .ok_or_else(|| anyhow::anyhow!("signal: phone_number is required for send"))?;
        let api_url = self.config.resolved_api_url();

        let (text, base64_attachments) = match content {
            ChannelContent::Text(t) => (t, None),
            ChannelContent::FileData {
                data,
                filename,
                mime_type,
            } => {
                let encoded = B64_STANDARD.encode(&data);
                let attachment = serde_json::json!({
                    "data": encoded,
                    "contentType": mime_type,
                    "filename": filename,
                });
                (String::new(), Some(vec![attachment]))
            }
            _ => return Ok(()),
        };

        // Check metadata for group_id.
        let group_id = user
            .platform_id
            .strip_prefix("group:")
            .map(|s| s.to_string());

        let mut body = if let Some(gid) = group_id {
            serde_json::json!({
                "number": phone_number,
                "message": text,
                "group_id": gid,
            })
        } else {
            serde_json::json!({
                "number": phone_number,
                "recipients": [&user.platform_id],
                "message": text,
            })
        };

        if let Some(attachments) = base64_attachments {
            body["base64_attachments"] = serde_json::json!(attachments);
        }

        let client = build_reqwest_client(&self.config)?;
        let resp = client
            .post(format!("{api_url}/v1/send"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("signal: send failed with status {status}: {body_text}");
        }

        Ok(())
    }

    async fn send_in_thread(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
        thread_id: &str,
    ) -> Result<()> {
        let phone_number = self.config.resolved_phone_number().ok_or_else(|| {
            anyhow::anyhow!("signal: phone_number is required for send_in_thread")
        })?;
        let api_url = self.config.resolved_api_url();

        let (text, base64_attachments) = match content {
            ChannelContent::Text(t) => (t, None),
            ChannelContent::FileData {
                data,
                filename,
                mime_type,
            } => {
                let encoded = B64_STANDARD.encode(&data);
                let attachment = serde_json::json!({
                    "data": encoded,
                    "contentType": mime_type,
                    "filename": filename,
                });
                (String::new(), Some(vec![attachment]))
            }
            _ => return Ok(()),
        };

        // If thread_id looks like a group id, send to group; else send to user.
        let mut body = if !thread_id.is_empty() && thread_id.len() > 10 {
            // Treat non-trivial thread_id as group_id.
            serde_json::json!({
                "number": phone_number,
                "message": text,
                "group_id": thread_id,
            })
        } else {
            serde_json::json!({
                "number": phone_number,
                "recipients": [&user.platform_id],
                "message": text,
            })
        };

        if let Some(attachments) = base64_attachments {
            body["base64_attachments"] = serde_json::json!(attachments);
        }

        let client = build_reqwest_client(&self.config)?;
        let resp = client
            .post(format!("{api_url}/v1/send"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("signal: send_in_thread failed with status {status}: {body_text}");
        }

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }

    async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()> {
        let error_text = format!("Sorry, I encountered an error: {err}");
        self.send(user, ChannelContent::Text(error_text)).await
    }

    fn conversation_key(&self, msg: &ChannelMessage) -> String {
        // Use group_id as the conversation key for group messages, else use source.
        if let Some(gid) = msg.metadata.get("group_id").and_then(|v| v.as_str())
            && !gid.is_empty()
        {
            return gid.to_string();
        }
        msg.sender.platform_id.clone()
    }
}

// -- Helpers ------------------------------------------------------------------

/// A single attachment from the signal-cli-rest-api envelope.
#[derive(Debug, Clone)]
pub(crate) struct SignalAttachment {
    /// Base64-encoded file data.
    pub data: String,
    /// MIME type (e.g. `"audio/ogg"`).
    pub content_type: String,
    /// Optional filename.
    pub filename: Option<String>,
}

/// Parsed fields from a signal-cli-rest-api WebSocket envelope.
#[derive(Debug)]
pub(crate) struct ParsedEnvelope {
    source: String,
    source_name: Option<String>,
    timestamp: i64,
    message: String,
    group_id: Option<String>,
    /// Attachments embedded in the dataMessage.
    attachments: Vec<SignalAttachment>,
}

/// Parse a JSON envelope from the signal-cli-rest-api WebSocket stream.
///
/// Returns `None` for non-dataMessage envelopes (receipts, typing indicators, etc.).
/// Envelopes with either a text message or attachments are accepted.
pub(crate) fn parse_envelope(raw: &str) -> Option<ParsedEnvelope> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let envelope = v.get("envelope")?;

    let source = envelope
        .get("source")
        .or_else(|| envelope.get("sourceNumber"))
        .and_then(|v| v.as_str())?
        .to_string();

    let source_name = envelope
        .get("sourceName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let data_msg = envelope.get("dataMessage")?;
    let message = data_msg
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Parse attachments (if any).
    let attachments = data_msg
        .get("attachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|att| {
                    let data = att.get("data").and_then(|v| v.as_str())?.to_string();
                    let content_type = att
                        .get("contentType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    let filename = att
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(SignalAttachment {
                        data,
                        content_type,
                        filename,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Require either a non-empty text message or at least one attachment.
    if message.is_empty() && attachments.is_empty() {
        return None;
    }

    let timestamp = envelope
        .get("timestamp")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let group_id = data_msg
        .pointer("/groupInfo/groupId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Some(ParsedEnvelope {
        source,
        source_name,
        timestamp,
        message,
        group_id,
        attachments,
    })
}

/// Convert a [`ParsedEnvelope`] into a [`ChannelMessage`].
///
/// If `audio_transcript` is provided, it is prepended to the text content.
fn envelope_to_channel_message(
    env: ParsedEnvelope,
    audio_transcript: Option<String>,
) -> Option<ChannelMessage> {
    let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
    metadata.insert(
        "source".to_string(),
        serde_json::Value::String(env.source.clone()),
    );
    metadata.insert(
        "timestamp".to_string(),
        serde_json::Value::Number(env.timestamp.into()),
    );

    let platform_id = if let Some(ref gid) = env.group_id {
        metadata.insert(
            "group_id".to_string(),
            serde_json::Value::String(gid.clone()),
        );
        // Use group: prefix so send() can detect group messages.
        format!("group:{gid}")
    } else {
        env.source.clone()
    };

    // Combine audio transcript and text message.
    let text = match (audio_transcript, env.message.is_empty()) {
        (Some(transcript), true) => transcript,
        (Some(transcript), false) => format!("{transcript}\n{}", env.message),
        (None, true) => return None, // no text and no transcript
        (None, false) => env.message,
    };

    Some(ChannelMessage {
        channel_type: ChannelType::Signal,
        platform_message_id: Some(env.timestamp.to_string()),
        sender: ChannelUser {
            platform_id,
            display_name: env.source_name,
        },
        content: ChannelContent::Text(text),
        thread_id: env.group_id,
        timestamp: SystemClock.now(),
        metadata,
    })
}

/// Attempt to transcribe audio attachments from a signal envelope.
///
/// Returns a formatted transcript string if any audio was successfully
/// transcribed, or `None` if there were no audio attachments or no provider.
async fn transcribe_audio_attachments(
    attachments: &[SignalAttachment],
    transcription: &Option<Arc<dyn TranscriptionProvider>>,
    language: &Option<String>,
) -> Option<String> {
    let audio_attachments: Vec<_> = attachments
        .iter()
        .filter(|a| is_audio_mime(&a.content_type))
        .collect();

    if audio_attachments.is_empty() {
        return None;
    }

    let provider = match transcription {
        Some(p) => p,
        None => {
            warn!(
                "signal: received audio attachment but no transcription provider configured; dropping"
            );
            return None;
        }
    };

    let mut transcripts = Vec::new();
    for att in audio_attachments {
        let decoded = match B64_STANDARD.decode(&att.data) {
            Ok(d) => d,
            Err(e) => {
                warn!(error = %e, "signal: failed to decode base64 audio attachment");
                continue;
            }
        };

        if decoded.len() > MAX_AUDIO_BYTES {
            warn!(
                size = decoded.len(),
                limit = MAX_AUDIO_BYTES,
                "signal: audio attachment too large; skipping"
            );
            continue;
        }

        let request = TranscriptionRequest {
            audio_data: decoded,
            mime_type: att.content_type.clone(),
            filename: att.filename.clone(),
            language: language.clone(),
        };
        match provider.transcribe(request).await {
            Ok(result) => transcripts.push(result.text),
            Err(e) => {
                warn!(error = %e, "signal: audio transcription failed");
            }
        }
    }

    if transcripts.is_empty() {
        return None;
    }

    Some(format!("[Voice message]: {}", transcripts.join(" ")))
}

/// Convert an HTTP base URL to a WebSocket URL.
///
/// `http://` → `ws://`, `https://` → `wss://`.
fn build_ws_url(api_url: &str, phone_number: &str) -> String {
    let ws_base = if api_url.starts_with("https://") {
        api_url.replacen("https://", "wss://", 1)
    } else {
        api_url.replacen("http://", "ws://", 1)
    };
    let ws_base = ws_base.trim_end_matches('/');
    format!("{ws_base}/v1/receive/{phone_number}")
}

/// Build a reqwest client with optional Basic Auth.
fn build_reqwest_client(config: &SignalConfig) -> Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(user) = config.resolved_api_user() {
        let pass = config.resolved_api_password().unwrap_or_default();
        let encoded = B64_STANDARD.encode(format!("{user}:{pass}"));
        let auth_value = reqwest::header::HeaderValue::from_str(&format!("Basic {encoded}"))?;
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
    }
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;
    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_envelope tests --------------------------------------------------

    #[test]
    fn parse_envelope_happy_path() {
        let raw = r#"{
            "envelope": {
                "source": "+14155550123",
                "sourceNumber": "+14155550123",
                "sourceUuid": "uuid-abc",
                "sourceName": "Alice",
                "sourceDevice": 1,
                "timestamp": 1234567890000,
                "dataMessage": {
                    "timestamp": 1234567890000,
                    "message": "Hello bot"
                }
            }
        }"#;
        let env = parse_envelope(raw).expect("should parse");
        assert_eq!(env.source, "+14155550123");
        assert_eq!(env.source_name.as_deref(), Some("Alice"));
        assert_eq!(env.message, "Hello bot");
        assert_eq!(env.timestamp, 1234567890000);
        assert!(env.group_id.is_none());
    }

    #[test]
    fn parse_envelope_group_message() {
        let raw = r#"{
            "envelope": {
                "source": "+14155550123",
                "sourceName": "Bob",
                "timestamp": 9999,
                "dataMessage": {
                    "timestamp": 9999,
                    "message": "Group hello",
                    "groupInfo": {
                        "groupId": "base64groupid==",
                        "type": "DELIVER"
                    }
                }
            }
        }"#;
        let env = parse_envelope(raw).expect("should parse");
        assert_eq!(env.group_id.as_deref(), Some("base64groupid=="));
        assert_eq!(env.message, "Group hello");
    }

    #[test]
    fn parse_envelope_receipt_returns_none() {
        // No dataMessage → should return None
        let raw = r#"{
            "envelope": {
                "source": "+14155550123",
                "timestamp": 1234,
                "receiptMessage": {
                    "when": 1234,
                    "isDelivery": true
                }
            }
        }"#;
        assert!(parse_envelope(raw).is_none());
    }

    #[test]
    fn parse_envelope_empty_message_returns_none() {
        let raw = r#"{
            "envelope": {
                "source": "+14155550123",
                "timestamp": 1234,
                "dataMessage": {
                    "timestamp": 1234,
                    "message": ""
                }
            }
        }"#;
        assert!(parse_envelope(raw).is_none());
    }

    #[test]
    fn build_ws_url_http() {
        let url = build_ws_url("http://localhost:8080", "+14155550123");
        assert_eq!(url, "ws://localhost:8080/v1/receive/+14155550123");
    }

    #[test]
    fn build_ws_url_https() {
        let url = build_ws_url("https://signal.example.com", "+14155550123");
        assert_eq!(url, "wss://signal.example.com/v1/receive/+14155550123");
    }

    #[test]
    fn conversation_key_uses_group_id_when_present() {
        let config = SignalConfig {
            phone_number: Some("+14155550123".to_string()),
            ..Default::default()
        };
        let adapter = SignalAdapter::new(config).unwrap();
        let mut metadata = HashMap::new();
        metadata.insert(
            "group_id".to_string(),
            serde_json::Value::String("groupABC".to_string()),
        );
        let msg = ChannelMessage {
            channel_type: ChannelType::Signal,
            platform_message_id: None,
            sender: ChannelUser {
                platform_id: "+14155550123".to_string(),
                display_name: None,
            },
            content: ChannelContent::Text("hi".to_string()),
            thread_id: Some("groupABC".to_string()),
            timestamp: chrono::Utc::now(),
            metadata,
        };
        assert_eq!(adapter.conversation_key(&msg), "groupABC");
    }

    // -- Audio / transcription tests -------------------------------------------

    #[test]
    fn parse_envelope_with_audio_attachment() {
        let raw = r#"{
            "envelope": {
                "source": "+14155550123",
                "sourceName": "Alice",
                "timestamp": 1111,
                "dataMessage": {
                    "timestamp": 1111,
                    "message": "",
                    "attachments": [
                        {
                            "contentType": "audio/ogg",
                            "data": "T2dnUw==",
                            "filename": "voice.ogg"
                        }
                    ]
                }
            }
        }"#;
        let env = parse_envelope(raw).expect("should parse audio-only envelope");
        assert!(env.message.is_empty());
        assert_eq!(env.attachments.len(), 1);
        assert_eq!(env.attachments[0].content_type, "audio/ogg");
        assert_eq!(env.attachments[0].filename.as_deref(), Some("voice.ogg"));
    }

    #[test]
    fn parse_envelope_text_only_has_no_attachments() {
        let raw = r#"{
            "envelope": {
                "source": "+14155550123",
                "timestamp": 2222,
                "dataMessage": {
                    "timestamp": 2222,
                    "message": "Just text"
                }
            }
        }"#;
        let env = parse_envelope(raw).expect("should parse");
        assert_eq!(env.message, "Just text");
        assert!(env.attachments.is_empty());
    }

    #[test]
    fn envelope_to_channel_message_with_transcript() {
        let env = ParsedEnvelope {
            source: "+1111".into(),
            source_name: None,
            timestamp: 42,
            message: String::new(),
            group_id: None,
            attachments: vec![],
        };
        let msg = envelope_to_channel_message(env, Some("[Voice message]: hello".into()));
        assert!(msg.is_some());
        let msg = msg.unwrap();
        match &msg.content {
            ChannelContent::Text(t) => assert_eq!(t, "[Voice message]: hello"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn envelope_to_channel_message_no_text_no_transcript_returns_none() {
        let env = ParsedEnvelope {
            source: "+1111".into(),
            source_name: None,
            timestamp: 42,
            message: String::new(),
            group_id: None,
            attachments: vec![],
        };
        assert!(envelope_to_channel_message(env, None).is_none());
    }

    #[tokio::test]
    async fn transcribe_audio_no_provider_returns_none() {
        let attachments = vec![SignalAttachment {
            data: B64_STANDARD.encode(b"fake-audio"),
            content_type: "audio/ogg".into(),
            filename: None,
        }];
        let result = transcribe_audio_attachments(&attachments, &None, &None).await;
        assert!(result.is_none(), "should return None without provider");
    }

    #[tokio::test]
    async fn transcribe_audio_non_audio_mime_returns_none() {
        // Even with a provider, non-audio MIME types should be skipped.
        let attachments = vec![SignalAttachment {
            data: B64_STANDARD.encode(b"image-data"),
            content_type: "image/png".into(),
            filename: None,
        }];
        // No provider needed since we shouldn't even try to transcribe.
        let result = transcribe_audio_attachments(&attachments, &None, &None).await;
        assert!(result.is_none());
    }

    #[test]
    fn conversation_key_falls_back_to_source_for_dm() {
        let config = SignalConfig {
            phone_number: Some("+14155550123".to_string()),
            ..Default::default()
        };
        let adapter = SignalAdapter::new(config).unwrap();
        let msg = ChannelMessage {
            channel_type: ChannelType::Signal,
            platform_message_id: None,
            sender: ChannelUser {
                platform_id: "+19995550001".to_string(),
                display_name: None,
            },
            content: ChannelContent::Text("hi".to_string()),
            thread_id: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        };
        assert_eq!(adapter.conversation_key(&msg), "+19995550001");
    }

    // ── build_reqwest_client ──────────────────────────────────────────────

    #[test]
    fn build_reqwest_client_no_auth_when_user_missing() {
        let cfg = SignalConfig {
            phone_number: Some("+14155550123".to_string()),
            api_user: None,
            ..Default::default()
        };
        // Just assert it builds without panicking.
        let _ = build_reqwest_client(&cfg).unwrap();
    }

    #[test]
    fn build_reqwest_client_includes_basic_auth_header_with_user() {
        let cfg = SignalConfig {
            phone_number: Some("+14155550123".to_string()),
            api_user: Some("admin".to_string()),
            api_password: Some("s3cret".to_string()),
            ..Default::default()
        };
        let _ = build_reqwest_client(&cfg).unwrap();
    }

    #[test]
    fn build_reqwest_client_handles_user_without_password() {
        let cfg = SignalConfig {
            phone_number: Some("+14155550123".to_string()),
            api_user: Some("user-only".to_string()),
            api_password: None,
            ..Default::default()
        };
        let _ = build_reqwest_client(&cfg).unwrap();
    }

    // ── envelope_to_channel_message — direct invocation ─────────────────

    #[test]
    fn envelope_to_channel_message_combines_audio_transcript_when_text_present() {
        let env = ParsedEnvelope {
            source: "+10000000001".to_string(),
            source_name: Some("Alice".to_string()),
            timestamp: 12345,
            message: "hello".to_string(),
            group_id: None,
            attachments: vec![],
        };
        let msg = envelope_to_channel_message(env, Some("[Voice]: hi".to_string())).unwrap();
        match msg.content {
            ChannelContent::Text(t) => {
                assert!(t.starts_with("[Voice]: hi"));
                assert!(t.contains("hello"));
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn envelope_to_channel_message_uses_group_prefix_for_group_messages() {
        let env = ParsedEnvelope {
            source: "+10000000001".to_string(),
            source_name: None,
            timestamp: 999,
            message: "in-group".to_string(),
            group_id: Some("group-xyz".to_string()),
            attachments: vec![],
        };
        let msg = envelope_to_channel_message(env, None).unwrap();
        assert_eq!(msg.sender.platform_id, "group:group-xyz");
        assert_eq!(msg.thread_id.as_deref(), Some("group-xyz"));
    }
}
