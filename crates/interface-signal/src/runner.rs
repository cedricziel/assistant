//! Signal interface runner.
//!
//! Without `--features signal` this provides a no-op [`SignalInterface`] that
//! returns an informative error when started.
//!
//! With `--features signal` the runner opens the SQLite store, loads the
//! registered presage [`Manager`], and enters a receive loop.  Each incoming
//! text message is dispatched synchronously to the [`Orchestrator`] and
//! the reply is sent back to the sender via the Signal protocol.
//!
//! # Architecture
//!
//! `presage::Manager::receive_messages()` takes `&mut self` to initialise the
//! WebSocket pipe but the returned `Stream` is owned — it does **not** borrow
//! from the manager.  This means the same `Manager` can be used for sending
//! inside the receive loop, which is the pattern used by `presage-cli`.
//!
//! # Safety
//!
//! `SignalConfig::allowed_senders` is checked before dispatching. Tools that
//! require explicit confirmation are auto-denied because Signal turns cannot
//! prompt interactively.

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::Orchestrator;

use crate::config::SignalConfig;
#[cfg(feature = "signal")]
use crate::config::SignalConfigExt;

/// The Signal interface handle.
pub struct SignalInterface {
    #[allow(dead_code)]
    config: SignalConfig,
    #[allow(dead_code)]
    orchestrator: Arc<Orchestrator>,
}

impl SignalInterface {
    /// Create a new [`SignalInterface`].
    ///
    /// Call [`run`][Self::run] to start the listener loop (requires
    /// `--features signal`).
    pub fn new(config: SignalConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Signal listener loop.
    ///
    /// Without `--features signal` this always returns an error explaining how
    /// to enable the feature.
    pub async fn run(&self) -> Result<()> {
        #[cfg(not(feature = "signal"))]
        {
            anyhow::bail!(
                "The Signal interface requires recompiling with `--features signal`.\n\
                 Rebuild with:\n\
                 \n\
                 cargo build -p assistant-interface-signal --features signal\n\
                 \n\
                 See crates/interface-signal/Cargo.toml for the presage git dependencies."
            );
        }

        #[cfg(feature = "signal")]
        self.run_presage_loop().await
    }

    /// The presage-backed receive loop (only compiled with `--features signal`).
    ///
    /// Processes one message at a time — this keeps the borrow structure
    /// simple: the receive stream and the manager are used sequentially.
    #[cfg(feature = "signal")]
    async fn run_presage_loop(&self) -> Result<()> {
        use futures::{pin_mut, StreamExt};
        use presage::{model::messages::Received, Manager};
        use presage_store_sqlite::{OnNewIdentity, SqliteConnectOptions, SqliteStore};
        use std::str::FromStr as _;
        use tracing::{debug, info, warn};

        use std::collections::HashMap;

        use assistant_core::Interface;
        use assistant_runtime::start_conversation_context;
        use opentelemetry::Context as OtelContext;
        use uuid::Uuid;

        // ── Graceful shutdown ─────────────────────────────────────────────────
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = sigterm.recv() => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            info!("Shutdown signal received, stopping…");
            let _ = shutdown_tx.send(true);
        });

        let store_path = self.config.resolved_store_path();
        info!(store_path = %store_path.display(), "Opening signal store");

        // Ensure the parent directory exists before SQLite tries to create the file.
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create signal store directory: {e}"))?;
        }

        // create_if_missing must be set explicitly — the default is false,
        // causing SQLITE_CANTOPEN when the file does not yet exist.
        let db_url = format!("sqlite://{}", store_path.display());
        let options = SqliteConnectOptions::from_str(&db_url)
            .map_err(|e| anyhow::anyhow!("Invalid signal store path: {e}"))?
            .create_if_missing(true);
        let store = SqliteStore::open_with_options(options, OnNewIdentity::Trust)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to open signal store: {e}"))?;

        let mut manager = Manager::load_registered(store)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load registered device: {e}"))?;

        info!("Signal manager loaded; entering receive loop");

        // `receive_messages` takes &mut self to open the WebSocket, but the
        // returned Stream is owned — the borrow ends after the `.await`.
        // The manager is therefore free to use for sending inside the loop.
        let messages = manager
            .receive_messages()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to start message stream: {e}"))?;
        pin_mut!(messages);

        // Track one (conversation_id, OtelContext) per sender so the
        // orchestrator retains memory across messages from the same Signal
        // contact and all turns share a single conversation-level trace.
        let mut conversations: HashMap<String, (Uuid, OtelContext)> = HashMap::new();

        loop {
            let maybe_received = tokio::select! {
                biased;
                _ = shutdown_rx.changed() => {
                    info!("Shutdown signal received, exiting receive loop");
                    break;
                }
                msg = messages.next() => msg,
            };

            let received = match maybe_received {
                Some(r) => r,
                None => {
                    info!("Signal message stream ended");
                    break;
                }
            };

            match received {
                Received::QueueEmpty => {
                    debug!("Initial message queue drained — listening for new messages");
                }
                Received::Contacts => {
                    debug!("Contact sync received");
                }
                Received::Content(content) => {
                    let sender = content.metadata.sender;
                    let text = extract_text_body(&content);

                    if text.is_empty() {
                        debug!("Ignoring non-text or empty message");
                        continue;
                    }

                    // Sender string for allowlist comparison.
                    let sender_str = sender.service_id_string();

                    // Allowlist check.
                    if !self.config.allowed_senders.is_empty()
                        && !self.config.allowed_senders.contains(&sender_str)
                    {
                        warn!(
                            sender = sender_str,
                            "Ignoring message from non-allowlisted sender"
                        );
                        continue;
                    }

                    info!(
                        sender = sender_str,
                        text_len = text.len(),
                        "Dispatching to orchestrator"
                    );

                    // Submit through the message bus with token streaming via
                    // a registered side-channel.
                    let (tok_tx, mut tok_rx) = tokio::sync::mpsc::channel::<String>(64);
                    let (conversation_id, _conv_cx) =
                        conversations.entry(sender_str.clone()).or_insert_with(|| {
                            let id = Uuid::new_v4();
                            let cx = start_conversation_context(id, &Interface::Signal);
                            (id, cx)
                        });
                    let conversation_id = *conversation_id;

                    let collector = tokio::spawn(async move {
                        let mut buf = String::new();
                        while let Some(tok) = tok_rx.recv().await {
                            buf.push_str(&tok);
                        }
                        buf
                    });

                    // Register the token sink so the worker streams to it.
                    self.orchestrator
                        .register_token_sink(conversation_id, tok_tx)
                        .await;

                    let orchestrator_start = std::time::Instant::now();
                    let turn_result = self
                        .orchestrator
                        .submit_turn(&text, conversation_id, Interface::Signal)
                        .await;
                    let elapsed_ms = orchestrator_start.elapsed().as_millis();

                    let reply = collector.await.unwrap_or_default();

                    if let Err(e) = turn_result {
                        tracing::error!(error = %e, elapsed_ms, "Orchestrator error");
                        continue;
                    }

                    if reply.is_empty() {
                        continue;
                    }

                    info!(
                        sender = sender_str,
                        elapsed_ms,
                        reply_len = reply.len(),
                        "Sending reply"
                    );

                    // Reply to the sender — use current wall-clock time in
                    // milliseconds (Signal's timestamp unit), not the sender's
                    // timestamp + 1 which would be incorrect.
                    let reply_ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let data_message = presage::proto::DataMessage {
                        body: Some(reply),
                        timestamp: Some(reply_ts),
                        ..Default::default()
                    };

                    if let Err(e) = manager.send_message(sender, data_message, reply_ts).await {
                        tracing::error!(error = %e, "Failed to send reply");
                    }
                }
            }
        }

        info!("Signal interface stopped");
        Ok(())
    }
}

/// Extract the plaintext body from a presage [`Content`] message.
///
/// Returns an empty string for non-data messages (calls, receipts, sync, …).
#[cfg(feature = "signal")]
fn extract_text_body(content: &presage::libsignal_service::content::Content) -> String {
    use presage::libsignal_service::content::ContentBody;
    match &content.body {
        ContentBody::DataMessage(msg) => msg.body.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::SignalConfig;

    // NOTE: We cannot safely instantiate `SignalInterface` in unit tests without
    // a full async runtime + storage stack (Orchestrator requires LLM, DB, …).
    // The tests below focus on the pure logic that is extractable without that
    // dependency — allowlist filtering and config defaults.
    //
    // The stub-mode error for `run()` is covered by the integration smoke-test
    // that builds the binary without `--features signal` and verifies the exit.

    #[test]
    fn allowlist_logic_empty_accepts_all() {
        let cfg = SignalConfig {
            allowed_senders: vec![],
            ..Default::default()
        };
        // Empty allowlist → every sender is accepted.
        let sender = "some-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&sender);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_logic_non_empty_blocks_unknown() {
        let cfg = SignalConfig {
            allowed_senders: vec!["allowed-uuid".to_string()],
            ..Default::default()
        };
        let unknown = "unknown-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_logic_non_empty_passes_known() {
        let cfg = SignalConfig {
            allowed_senders: vec!["allowed-uuid".to_string()],
            ..Default::default()
        };
        let known = "allowed-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&known);
        assert!(!blocked);
    }
}
