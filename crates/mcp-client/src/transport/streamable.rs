//! Streamable HTTP transport — the newer MCP HTTP transport.
//!
//! Uses a single HTTP endpoint where the client POSTs JSON-RPC requests and
//! the server responds with a stream of JSON-RPC messages. Session state is
//! tracked via a `Mcp-Session-Id` header.
//!
//! Key differences from the SSE transport:
//! - No separate SSE endpoint — one URL handles everything.
//! - The response to a POST can be a single JSON object or a stream (SSE).
//! - The server may send an `Mcp-Session-Id` header to establish a session.
//! - The client can open a GET request for server-initiated notifications.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::sync::{broadcast, oneshot, Mutex, RwLock};
use tracing::{debug, error, trace, warn};

use crate::protocol::{JsonRpcMessage, JsonRpcRequest};
use crate::transport::McpTransport;

/// Streamable HTTP MCP transport.
pub struct StreamableHttpTransport {
    /// HTTP client.
    http: reqwest::Client,
    /// The server endpoint URL.
    url: String,
    /// Extra headers.
    headers: HeaderMap,
    /// Session ID assigned by the server (if any).
    session_id: RwLock<Option<String>>,
    /// Pending request waiters: `id → oneshot sender`.
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>>,
    /// Broadcast channel for server-initiated notifications.
    notification_tx: broadcast::Sender<JsonRpcMessage>,
    /// Whether the transport is still alive.
    connected: Arc<AtomicBool>,
    /// Handle to the optional notification listener (GET) task.
    listener_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl StreamableHttpTransport {
    /// Create a new Streamable HTTP transport.
    ///
    /// Unlike the SSE transport, this doesn't need an initial connection
    /// to discover endpoints — the URL is used directly for all requests.
    pub async fn connect(url: &str, extra_headers: &HashMap<String, String>) -> Result<Self> {
        let mut header_map = HeaderMap::new();
        for (k, v) in extra_headers {
            let name = HeaderName::from_bytes(k.as_bytes())
                .with_context(|| format!("invalid header name: {k}"))?;
            let value = HeaderValue::from_str(v)
                .with_context(|| format!("invalid header value for {k}"))?;
            header_map.insert(name, value);
        }

        let http = reqwest::Client::builder()
            .default_headers(header_map.clone())
            .build()
            .context("failed to build HTTP client")?;

        let (notification_tx, _) = broadcast::channel(64);
        let connected = Arc::new(AtomicBool::new(true));

        debug!(url = %url, "streamable HTTP transport created");

        Ok(Self {
            http,
            url: url.to_string(),
            headers: header_map,
            session_id: RwLock::new(None),
            pending: Arc::new(RwLock::new(HashMap::new())),
            notification_tx,
            connected,
            listener_handle: Mutex::new(None),
        })
    }

    /// Build the request headers including session ID if present.
    async fn build_headers(&self) -> HeaderMap {
        let mut headers = self.headers.clone();
        if let Some(ref sid) = *self.session_id.read().await {
            if let Ok(val) = HeaderValue::from_str(sid) {
                headers.insert("mcp-session-id", val);
            }
        }
        headers
    }

    /// Extract and store the session ID from a response.
    async fn update_session_id(&self, response: &reqwest::Response) {
        if let Some(sid) = response.headers().get("mcp-session-id") {
            if let Ok(s) = sid.to_str() {
                let mut session = self.session_id.write().await;
                if session.as_deref() != Some(s) {
                    debug!(session_id = %s, "MCP session established");
                    *session = Some(s.to_string());
                }
            }
        }
    }

    /// Parse a response body that could be either a single JSON object or
    /// an SSE stream of JSON objects.
    async fn parse_response(&self, response: reqwest::Response) -> Result<Vec<JsonRpcMessage>> {
        self.update_session_id(&response).await;

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("text/event-stream") {
            // SSE stream response — parse events.
            let mut messages = Vec::new();
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.context("SSE stream error")?;
                let text = std::str::from_utf8(&chunk).unwrap_or("");
                buffer.push_str(text);

                while let Some(pos) = buffer.find("\n\n") {
                    let event_block = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    for line in event_block.lines() {
                        if let Some(data) = line.strip_prefix("data:") {
                            let data = data.trim();
                            if !data.is_empty() {
                                match serde_json::from_str::<JsonRpcMessage>(data) {
                                    Ok(msg) => messages.push(msg),
                                    Err(e) => {
                                        warn!(error = %e, "invalid JSON in SSE data");
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Ok(messages)
        } else {
            // Single JSON response.
            let body = response
                .text()
                .await
                .context("failed to read response body")?;
            trace!(raw = %body, "streamable HTTP ← server");

            let msg: JsonRpcMessage =
                serde_json::from_str(&body).context("failed to parse JSON-RPC response")?;
            Ok(vec![msg])
        }
    }

    /// Start a background GET listener for server-initiated notifications.
    pub async fn start_notification_listener(&self) -> Result<()> {
        let mut guard = self.listener_handle.lock().await;
        if guard.is_some() {
            return Ok(()); // Already running.
        }

        let http = self.http.clone();
        let url = self.url.clone();
        let headers = self.build_headers().await;
        let notification_tx = self.notification_tx.clone();
        let pending = self.pending.clone();
        let connected = self.connected.clone();

        let handle = tokio::spawn(async move {
            let response = match http.get(&url).headers(headers).send().await {
                Ok(r) if r.status().is_success() => r,
                Ok(r) => {
                    warn!(status = %r.status(), "notification listener GET returned error");
                    return;
                }
                Err(e) => {
                    warn!(error = %e, "notification listener GET failed");
                    return;
                }
            };

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        error!(error = %e, "notification listener stream error");
                        connected.store(false, Ordering::SeqCst);
                        break;
                    }
                };

                let text = match std::str::from_utf8(&chunk) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                buffer.push_str(text);

                while let Some(pos) = buffer.find("\n\n") {
                    let event_block = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    for line in event_block.lines() {
                        if let Some(data) = line.strip_prefix("data:") {
                            let data = data.trim();
                            if data.is_empty() {
                                continue;
                            }
                            match serde_json::from_str::<JsonRpcMessage>(data) {
                                Ok(msg) => {
                                    if msg.is_notification() {
                                        let _ = notification_tx.send(msg);
                                    } else if let Some(id) = msg.request_id() {
                                        let mut pending = pending.write().await;
                                        if let Some(tx) = pending.remove(&id) {
                                            let _ = tx.send(msg);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!(error = %e, "invalid JSON in notification stream");
                                }
                            }
                        }
                    }
                }
            }

            debug!("notification listener stream ended");
        });

        *guard = Some(handle);
        Ok(())
    }
}

#[async_trait]
impl McpTransport for StreamableHttpTransport {
    async fn request(&self, req: JsonRpcRequest) -> Result<JsonRpcMessage> {
        let id = req
            .id
            .ok_or_else(|| anyhow::anyhow!("request must have an id"))?;

        let headers = self.build_headers().await;
        let json = serde_json::to_string(&req)?;
        trace!(raw = %json, url = %self.url, "streamable HTTP → server");

        let response = self
            .http
            .post(&self.url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(json)
            .send()
            .await
            .context("POST request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("POST request returned {status}: {body}");
        }

        let messages = self.parse_response(response).await?;

        // Find the response matching our request ID.
        for msg in messages {
            if msg.request_id() == Some(id) {
                return Ok(msg);
            }
            // Non-matching messages are notifications or responses to other requests.
            if msg.is_notification() {
                let _ = self.notification_tx.send(msg);
            } else if let Some(other_id) = msg.request_id() {
                let mut pending = self.pending.write().await;
                if let Some(tx) = pending.remove(&other_id) {
                    let _ = tx.send(msg);
                }
            }
        }

        anyhow::bail!("no response with id={id} in server reply")
    }

    async fn notify(&self, req: JsonRpcRequest) -> Result<()> {
        let headers = self.build_headers().await;
        let json = serde_json::to_string(&req)?;
        trace!(raw = %json, url = %self.url, "streamable HTTP → server (notification)");

        let resp = self
            .http
            .post(&self.url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(json)
            .send()
            .await
            .context("notification POST failed")?;

        self.update_session_id(&resp).await;

        if !resp.status().is_success() {
            warn!(status = %resp.status(), "notification POST returned non-success");
        }

        Ok(())
    }

    async fn start_notification_listener(&self) -> Result<()> {
        self.start_notification_listener().await
    }

    fn notifications(&self) -> broadcast::Receiver<JsonRpcMessage> {
        self.notification_tx.subscribe()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn shutdown(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);

        if let Some(handle) = self.listener_handle.lock().await.take() {
            handle.abort();
        }

        let mut pending = self.pending.write().await;
        pending.clear();

        Ok(())
    }
}
