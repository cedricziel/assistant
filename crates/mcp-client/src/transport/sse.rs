//! HTTP/SSE transport — connects to a remote MCP server via Server-Sent Events.
//!
//! The MCP SSE transport works as follows:
//! 1. Client opens an SSE connection (GET) to the server's SSE endpoint.
//! 2. The server sends an `endpoint` event with the URL to POST requests to.
//! 3. Client sends JSON-RPC requests via POST to that endpoint.
//! 4. Server sends responses and notifications back through the SSE stream.

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

/// HTTP/SSE-based MCP transport.
///
/// Connects to a server that exposes an SSE endpoint and a POST endpoint
/// for JSON-RPC messages.
pub struct SseTransport {
    /// HTTP client for POST requests.
    http: reqwest::Client,
    /// The POST endpoint URL discovered from the SSE `endpoint` event.
    post_url: RwLock<Option<String>>,
    /// Extra headers (retained for potential reconnection).
    #[allow(dead_code)]
    headers: HeaderMap,
    /// Pending request waiters: `id → oneshot sender`.
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>>,
    /// Broadcast channel for server-initiated notifications.
    notification_tx: broadcast::Sender<JsonRpcMessage>,
    /// Whether the transport is still alive.
    connected: Arc<AtomicBool>,
    /// Handle to the SSE reader task.
    reader_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SseTransport {
    /// Connect to an MCP server's SSE endpoint.
    ///
    /// `url` is the SSE endpoint URL (e.g. `https://example.com/mcp/sse`).
    /// `headers` are extra HTTP headers sent with every request (e.g. auth tokens).
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

        let pending: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let (notification_tx, _) = broadcast::channel(64);
        let connected = Arc::new(AtomicBool::new(true));
        let post_url: Arc<RwLock<Option<String>>> = Arc::new(RwLock::new(None));

        // Start the SSE reader task.
        let reader_handle = {
            let pending = pending.clone();
            let notification_tx = notification_tx.clone();
            let connected = connected.clone();
            let post_url_writer = post_url.clone();
            let sse_url = url.to_string();
            let http_clone = http.clone();

            tokio::spawn(async move {
                Self::sse_reader_loop(
                    &http_clone,
                    &sse_url,
                    post_url_writer,
                    pending,
                    notification_tx,
                    connected,
                )
                .await;
            })
        };

        // Wait for the `endpoint` event (up to 10 seconds).
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if post_url.read().await.is_some() {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!("timed out waiting for MCP SSE endpoint event from {url}");
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        debug!(url = %url, "SSE transport connected");

        // We need to extract the inner RwLock from the Arc for the struct.
        // Since we own the only strong reference besides the reader task,
        // we can clone the current value.
        let current_post_url = post_url.read().await.clone();
        let transport_post_url = RwLock::new(current_post_url);

        Ok(Self {
            http,
            post_url: transport_post_url,
            headers: header_map,
            pending,
            notification_tx,
            connected,
            reader_handle: Mutex::new(Some(reader_handle)),
        })
    }

    /// The SSE reader loop — runs in a background task.
    async fn sse_reader_loop(
        http: &reqwest::Client,
        sse_url: &str,
        post_url: Arc<RwLock<Option<String>>>,
        pending: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>>,
        notification_tx: broadcast::Sender<JsonRpcMessage>,
        connected: Arc<AtomicBool>,
    ) {
        let response = match http.get(sse_url).send().await {
            Ok(r) => r,
            Err(e) => {
                error!(error = %e, "failed to connect to SSE endpoint");
                connected.store(false, Ordering::SeqCst);
                return;
            }
        };

        if !response.status().is_success() {
            error!(
                status = %response.status(),
                "SSE endpoint returned non-success status"
            );
            connected.store(false, Ordering::SeqCst);
            return;
        }

        // Parse the base URL for resolving relative endpoint paths.
        let base_url = sse_url
            .rfind('/')
            .map(|i| &sse_url[..i + 1])
            .unwrap_or(sse_url);

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut event_type = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    error!(error = %e, "SSE stream error");
                    connected.store(false, Ordering::SeqCst);
                    break;
                }
            };

            let text = match std::str::from_utf8(&chunk) {
                Ok(t) => t,
                Err(_) => continue,
            };

            // SSE parsing: accumulate lines, process on double newline.
            buffer.push_str(text);

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                let mut data_lines = Vec::new();
                event_type.clear();

                for line in event_block.lines() {
                    if let Some(rest) = line.strip_prefix("event:") {
                        event_type = rest.trim().to_string();
                    } else if let Some(rest) = line.strip_prefix("data:") {
                        data_lines.push(rest.trim().to_string());
                    }
                }

                let data = data_lines.join("\n");
                if data.is_empty() {
                    continue;
                }

                match event_type.as_str() {
                    "endpoint" => {
                        // The server tells us where to POST requests.
                        let endpoint =
                            if data.starts_with("http://") || data.starts_with("https://") {
                                data
                            } else {
                                format!("{}{}", base_url, data.trim_start_matches('/'))
                            };
                        debug!(endpoint = %endpoint, "received SSE endpoint");
                        *post_url.write().await = Some(endpoint);
                    }
                    "message" | "" => {
                        // JSON-RPC message from the server.
                        trace!(raw = %data, "SSE ← server");
                        match serde_json::from_str::<JsonRpcMessage>(&data) {
                            Ok(msg) => {
                                if msg.is_notification() {
                                    let _ = notification_tx.send(msg);
                                } else if let Some(id) = msg.request_id() {
                                    let mut pending = pending.write().await;
                                    if let Some(tx) = pending.remove(&id) {
                                        let _ = tx.send(msg);
                                    } else {
                                        warn!(id, "received response for unknown request ID");
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, raw = %data, "invalid JSON from SSE");
                            }
                        }
                    }
                    other => {
                        debug!(event = %other, "ignoring unknown SSE event type");
                    }
                }
            }
        }

        debug!("SSE stream ended");
        connected.store(false, Ordering::SeqCst);

        // Wake up any pending requests.
        let mut pending = pending.write().await;
        pending.clear();
    }

    /// Get the current POST endpoint URL.
    async fn get_post_url(&self) -> Result<String> {
        self.post_url
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("SSE endpoint URL not yet received"))
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    async fn request(&self, req: JsonRpcRequest) -> Result<JsonRpcMessage> {
        let id = req
            .id
            .ok_or_else(|| anyhow::anyhow!("request must have an id"))?;
        let post_url = self.get_post_url().await?;

        // Register a waiter before sending.
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.write().await;
            pending.insert(id, tx);
        }

        // Send the request via POST.
        let json = serde_json::to_string(&req)?;
        trace!(raw = %json, url = %post_url, "SSE → server (POST)");

        let response = self
            .http
            .post(&post_url)
            .header("Content-Type", "application/json")
            .body(json)
            .send()
            .await;

        if let Err(e) = response {
            let mut pending = self.pending.write().await;
            pending.remove(&id);
            anyhow::bail!("POST request failed: {e}");
        }

        let resp = response.unwrap();
        if !resp.status().is_success() {
            let mut pending = self.pending.write().await;
            pending.remove(&id);
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("POST request returned {status}: {body}");
        }

        // Wait for the response via the SSE stream.
        match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(_)) => {
                anyhow::bail!("SSE response channel closed (connection lost)")
            }
            Err(_) => {
                let mut pending = self.pending.write().await;
                pending.remove(&id);
                anyhow::bail!("SSE request timed out after 60s")
            }
        }
    }

    async fn notify(&self, req: JsonRpcRequest) -> Result<()> {
        let post_url = self.get_post_url().await?;
        let json = serde_json::to_string(&req)?;
        trace!(raw = %json, url = %post_url, "SSE → server (POST notification)");

        let resp = self
            .http
            .post(&post_url)
            .header("Content-Type", "application/json")
            .body(json)
            .send()
            .await
            .context("POST notification failed")?;

        if !resp.status().is_success() {
            warn!(status = %resp.status(), "notification POST returned non-success");
        }

        Ok(())
    }

    fn notifications(&self) -> broadcast::Receiver<JsonRpcMessage> {
        self.notification_tx.subscribe()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn shutdown(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);

        if let Some(handle) = self.reader_handle.lock().await.take() {
            handle.abort();
        }

        // Clear pending waiters.
        let mut pending = self.pending.write().await;
        pending.clear();

        Ok(())
    }
}
