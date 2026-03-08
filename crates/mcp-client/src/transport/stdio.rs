//! Stdio transport — spawns a subprocess and communicates via JSON-RPC over
//! stdin/stdout.
//!
//! The child process receives JSON-RPC messages as newline-delimited JSON on
//! stdin and writes responses/notifications on stdout (also newline-delimited).
//! Stderr is forwarded to the `tracing` logger.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, oneshot, Mutex, RwLock};
use tracing::{debug, error, trace, warn};

use crate::protocol::{JsonRpcMessage, JsonRpcRequest};
use crate::transport::McpTransport;

/// Stdio-based MCP transport.
///
/// Manages a child process and routes JSON-RPC messages through its
/// stdin/stdout pipes.
pub struct StdioTransport {
    /// Sender half of stdin — guarded by a mutex so only one request writes at
    /// a time.
    stdin_tx: Mutex<Option<tokio::process::ChildStdin>>,
    /// Pending request waiters: `id → oneshot sender`.
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>>,
    /// Broadcast channel for server-initiated notifications.
    notification_tx: broadcast::Sender<JsonRpcMessage>,
    /// Whether the transport is still alive.
    connected: Arc<AtomicBool>,
    /// Handle to the reader task so we can abort on shutdown.
    reader_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Handle to the stderr logger task.
    stderr_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Child process — kept alive for the duration of the transport.
    child: Mutex<Option<Child>>,
}

impl StdioTransport {
    /// Spawn a subprocess and set up the stdio transport.
    ///
    /// `command` is the full command line (first element = program, rest = args).
    /// `env` contains extra environment variables for the child process.
    pub async fn spawn(command: &[String], env: &HashMap<String, String>) -> Result<Self> {
        if command.is_empty() {
            anyhow::bail!("MCP server command is empty");
        }

        let program = &command[0];
        let args = &command[1..];

        debug!(program = %program, args = ?args, "spawning MCP server subprocess");

        let mut cmd = Command::new(program);
        cmd.args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // Prevent the child from inheriting the parent's signal handlers.
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn MCP server: {program}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server stdin not captured"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server stdout not captured"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("MCP server stderr not captured"))?;

        let pending: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcMessage>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let (notification_tx, _) = broadcast::channel(64);
        let connected = Arc::new(AtomicBool::new(true));

        // Spawn stdout reader task.
        let reader_handle = {
            let pending = pending.clone();
            let notification_tx = notification_tx.clone();
            let connected = connected.clone();

            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut line = String::new();

                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => {
                            // EOF — child process closed stdout.
                            debug!("MCP server stdout closed (EOF)");
                            connected.store(false, Ordering::SeqCst);
                            break;
                        }
                        Ok(_) => {
                            let trimmed = line.trim();
                            if trimmed.is_empty() {
                                continue;
                            }
                            trace!(raw = %trimmed, "MCP ← server");

                            match serde_json::from_str::<JsonRpcMessage>(trimmed) {
                                Ok(msg) => {
                                    if msg.is_notification() {
                                        // Server-initiated notification.
                                        let _ = notification_tx.send(msg);
                                    } else if let Some(id) = msg.request_id() {
                                        // Response to a request we sent.
                                        let mut pending = pending.write().await;
                                        if let Some(tx) = pending.remove(&id) {
                                            let _ = tx.send(msg);
                                        } else {
                                            warn!(id, "received response for unknown request ID");
                                        }
                                    } else {
                                        warn!(raw = %trimmed, "unroutable JSON-RPC message");
                                    }
                                }
                                Err(e) => {
                                    warn!(error = %e, raw = %trimmed, "invalid JSON from MCP server");
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "error reading MCP server stdout");
                            connected.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }

                // Wake up any pending requests.
                let mut pending = pending.write().await;
                pending.clear();
            })
        };

        // Spawn stderr logger task.
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            debug!(stderr = %trimmed, "MCP server");
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            stdin_tx: Mutex::new(Some(stdin)),
            pending,
            notification_tx,
            connected,
            reader_handle: Mutex::new(Some(reader_handle)),
            stderr_handle: Mutex::new(Some(stderr_handle)),
            child: Mutex::new(Some(child)),
        })
    }

    /// Write a JSON-RPC message to the child's stdin.
    async fn write_message(&self, req: &JsonRpcRequest) -> Result<()> {
        let mut stdin_guard = self.stdin_tx.lock().await;
        let stdin = stdin_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("MCP transport stdin closed"))?;

        let json = serde_json::to_string(req)?;
        trace!(raw = %json, "MCP → server");

        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn request(&self, req: JsonRpcRequest) -> Result<JsonRpcMessage> {
        let id = req
            .id
            .ok_or_else(|| anyhow::anyhow!("request must have an id"))?;

        // Register a waiter before sending the request to avoid races.
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.write().await;
            pending.insert(id, tx);
        }

        // Send the request.
        if let Err(e) = self.write_message(&req).await {
            // Clean up the waiter on send failure.
            let mut pending = self.pending.write().await;
            pending.remove(&id);
            return Err(e);
        }

        // Wait for the response with a timeout.
        match tokio::time::timeout(std::time::Duration::from_secs(60), rx).await {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(_)) => {
                anyhow::bail!("MCP response channel closed (server may have exited)")
            }
            Err(_) => {
                // Clean up the waiter.
                let mut pending = self.pending.write().await;
                pending.remove(&id);
                anyhow::bail!("MCP request timed out after 60s")
            }
        }
    }

    async fn notify(&self, req: JsonRpcRequest) -> Result<()> {
        self.write_message(&req).await
    }

    fn notifications(&self) -> broadcast::Receiver<JsonRpcMessage> {
        self.notification_tx.subscribe()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn shutdown(&self) -> Result<()> {
        self.connected.store(false, Ordering::SeqCst);

        // Close stdin to signal the child.
        {
            let mut stdin = self.stdin_tx.lock().await;
            stdin.take();
        }

        // Abort reader tasks.
        if let Some(handle) = self.reader_handle.lock().await.take() {
            handle.abort();
        }
        if let Some(handle) = self.stderr_handle.lock().await.take() {
            handle.abort();
        }

        // Kill the child process.
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
        }

        Ok(())
    }
}
