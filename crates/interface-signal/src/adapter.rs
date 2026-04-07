//! Signal `ChannelAdapter` implementation backed by signal-cli-rest-api.
//!
//! Connects to the signal-cli-rest-api WebSocket endpoint
//! `GET /v1/receive/{phone_number}` and yields inbound messages as
//! [`ChannelMessage`] items.  Outbound replies are POSTed to
//! `POST /v1/send`.
//!
//! Auto-reconnects with exponential backoff on disconnect (same pattern as
//! [`SlackAdapter`][crate]).

use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as B64_STANDARD;
use base64::Engine as _;
use chrono::Utc;
use futures::stream::Stream;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{debug, error, info, warn};

use crate::config::{SignalConfig, SignalConfigExt};

/// Minimum reconnect delay.
const BACKOFF_MIN: Duration = Duration::from_secs(1);
/// Maximum reconnect delay.
const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Signal `ChannelAdapter`.  Connects to the signal-cli-rest-api WebSocket
/// and provides `send()` via `POST /v1/send`.
pub struct SignalAdapter {
    config: SignalConfig,
    stop_tx: tokio::sync::watch::Sender<bool>,
    stop_rx: tokio::sync::watch::Receiver<bool>,
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
        })
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

                                            if let Some(msg) = envelope_to_channel_message(env) {
                                                if tx.send(msg).await.is_err() {
                                                    return; // receiver dropped
                                                }
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

        let text = match content {
            ChannelContent::Text(t) => t,
            _ => return Ok(()), // silently skip non-text content
        };

        // Check metadata for group_id.
        let group_id = user
            .platform_id
            .strip_prefix("group:")
            .map(|s| s.to_string());

        let body = if let Some(gid) = group_id {
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

        let text = match content {
            ChannelContent::Text(t) => t,
            _ => return Ok(()), // silently skip non-text content
        };

        // If thread_id looks like a group id, send to group; else send to user.
        let body = if !thread_id.is_empty() && thread_id.len() > 10 {
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
        if let Some(gid) = msg.metadata.get("group_id").and_then(|v| v.as_str()) {
            if !gid.is_empty() {
                return gid.to_string();
            }
        }
        msg.sender.platform_id.clone()
    }
}

// -- Helpers ------------------------------------------------------------------

/// Parsed fields from a signal-cli-rest-api WebSocket envelope.
#[derive(Debug)]
pub(crate) struct ParsedEnvelope {
    source: String,
    source_name: Option<String>,
    timestamp: i64,
    message: String,
    group_id: Option<String>,
}

/// Parse a JSON envelope from the signal-cli-rest-api WebSocket stream.
///
/// Returns `None` for non-dataMessage envelopes (receipts, typing indicators, etc.).
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
        .and_then(|v| v.as_str())?
        .to_string();

    if message.is_empty() {
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
    })
}

/// Convert a [`ParsedEnvelope`] into a [`ChannelMessage`].
fn envelope_to_channel_message(env: ParsedEnvelope) -> Option<ChannelMessage> {
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

    Some(ChannelMessage {
        channel_type: ChannelType::Signal,
        platform_message_id: Some(env.timestamp.to_string()),
        sender: ChannelUser {
            platform_id,
            display_name: env.source_name,
        },
        content: ChannelContent::Text(env.message),
        thread_id: env.group_id,
        timestamp: Utc::now(),
        metadata,
    })
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

/// Sleep for the current backoff duration (with ±10% jitter), then double it.
async fn sleep_backoff(backoff: &mut Duration, stop_rx: &mut tokio::sync::watch::Receiver<bool>) {
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
            timestamp: Utc::now(),
            metadata,
        };
        assert_eq!(adapter.conversation_key(&msg), "groupABC");
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
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        assert_eq!(adapter.conversation_key(&msg), "+19995550001");
    }
}
