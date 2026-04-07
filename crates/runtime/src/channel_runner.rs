//! Generic async runner for any [`ChannelAdapter`].
//!
//! [`ChannelRunner`] drives the common dispatch loop that all messenger
//! interfaces share:
//!
//! 1. Call `adapter.start()` to obtain the inbound message stream.
//! 2. Run BOOT.md startup hook.
//! 3. For each message: resolve conversation UUID, serialise turns per
//!    conversation, invoke the orchestrator.
//! 4. Propagate lifecycle hooks (`on_turn_start`, `on_turn_success`,
//!    `on_turn_error`) back to the adapter.
//! 5. Shut down cleanly on SIGTERM / Ctrl-C (or an external cancel token).
//!
//! Platform-specific behaviour (tools, reactions, threading) lives entirely
//! inside the adapter's hook overrides — not here.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ChannelAdapter, ChannelContent, ChannelMessage};
use futures::StreamExt;
use lru::LruCache;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;

/// Type alias for the per-conversation serialisation map.
type ConvLocks = Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>;

/// Generic runner that drives any [`ChannelAdapter`] through the
/// orchestrator dispatch loop.
pub struct ChannelRunner {
    adapter: Arc<dyn ChannelAdapter>,
    orchestrator: Arc<Orchestrator>,
    /// Map `conversation_key → Uuid`.
    conversations: Arc<Mutex<LruCache<String, Uuid>>>,
    /// Per-conversation turn lock (serialises concurrent turns).
    conv_locks: ConvLocks,
    /// Optional external shutdown signal (background / daemon mode).
    shutdown: Option<tokio_util::sync::CancellationToken>,
}

impl ChannelRunner {
    /// Create a new runner for `adapter`.
    pub fn new(adapter: Arc<dyn ChannelAdapter>, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            adapter,
            orchestrator,
            conversations: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(10_000).unwrap(),
            ))),
            conv_locks: Arc::new(Mutex::new(HashMap::new())),
            shutdown: None,
        }
    }

    /// Attach an external shutdown token (for background / daemon mode).
    ///
    /// When set, the runner exits when the token is cancelled instead of
    /// installing process-wide signal handlers.
    pub fn with_shutdown(mut self, token: tokio_util::sync::CancellationToken) -> Self {
        self.shutdown = Some(token);
        self
    }

    /// Resolve or create a conversation UUID for `key`.
    async fn resolve_conv_id(
        conversations: &Arc<Mutex<LruCache<String, Uuid>>>,
        key: &str,
    ) -> Uuid {
        let mut convs = conversations.lock().await;
        if let Some(&id) = convs.get(key) {
            return id;
        }
        let id = Uuid::new_v4();
        convs.put(key.to_string(), id);
        id
    }

    /// Get (or create) the per-conversation serialisation lock.
    async fn get_conv_lock(conv_locks: &ConvLocks, conv_id: Uuid) -> Arc<Mutex<()>> {
        let mut locks = conv_locks.lock().await;
        locks
            .entry(conv_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Dispatch a single inbound message through the orchestrator.
    async fn dispatch(
        msg: ChannelMessage,
        conv_id: Uuid,
        adapter: Arc<dyn ChannelAdapter>,
        orchestrator: Arc<Orchestrator>,
    ) {
        // Only handle text content.
        let text = match &msg.content {
            ChannelContent::Text(t) => t.clone(),
            _ => return,
        };

        let user = msg.sender.clone();
        let thread_id = msg.thread_id.clone();
        let interface = adapter.interface();
        let tools = adapter.platform_tools(&msg, conv_id);

        // Lifecycle: turn starting.
        if let Err(e) = adapter.on_turn_start(&user, conv_id).await {
            warn!(adapter = adapter.name(), error = %e, "on_turn_start hook failed");
        }

        let result = orchestrator
            .run_turn_with_tools(&text, conv_id, interface, tools, None, vec![])
            .await;

        match result {
            Ok(turn_result) => {
                if let Err(e) = adapter
                    .on_turn_success(&user, &turn_result.answer, &turn_result.attachments)
                    .await
                {
                    warn!(adapter = adapter.name(), error = %e, "on_turn_success hook failed");
                }

                // If the turn produced an answer and it wasn't sent via a
                // platform-specific reply tool, send it now.
                if !turn_result.answer.is_empty() {
                    let content = ChannelContent::Text(turn_result.answer);
                    let send_result = if let Some(tid) = &thread_id {
                        adapter.send_in_thread(&user, content, tid).await
                    } else {
                        adapter.send(&user, content).await
                    };
                    if let Err(e) = send_result {
                        warn!(adapter = adapter.name(), error = %e, "failed to send reply");
                    }
                }
            }
            Err(ref e) => {
                error!(
                    adapter = adapter.name(),
                    conv_id = %conv_id,
                    error = %e,
                    "orchestrator turn failed"
                );
                if let Err(he) = adapter.on_turn_error(&user, e).await {
                    warn!(adapter = adapter.name(), error = %he, "on_turn_error hook failed");
                }
            }
        }
    }

    /// Run the adapter dispatch loop until shutdown.
    pub async fn run(self) -> Result<()> {
        let name = self.adapter.name().to_string();

        // BOOT.md startup hook (non-fatal).
        let boot_id = Uuid::new_v4();
        let interface = self.adapter.interface();
        if let Err(e) = self.orchestrator.run_boot(boot_id, interface).await {
            warn!(adapter = %name, error = %e, "BOOT hook failed (non-fatal)");
        }

        let mut stream = self.adapter.start().await?;
        info!(adapter = %name, "connected and listening");

        // Build a shutdown future.
        let shutdown_fut: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> =
            if let Some(token) = &self.shutdown {
                let token = token.clone();
                Box::pin(async move { token.cancelled().await })
            } else {
                Box::pin(Self::default_shutdown_signal())
            };
        tokio::pin!(shutdown_fut);

        loop {
            tokio::select! {
                () = &mut shutdown_fut => {
                    info!(adapter = %name, "shutdown signal received");
                    break;
                }
                msg = stream.next() => {
                    match msg {
                        None => {
                            warn!(adapter = %name, "message stream ended");
                            break;
                        }
                        Some(channel_msg) => {
                            let key = self.adapter.conversation_key(&channel_msg);
                            let conv_id =
                                Self::resolve_conv_id(&self.conversations, &key).await;
                            let lock =
                                Self::get_conv_lock(&self.conv_locks, conv_id).await;

                            let adapter = self.adapter.clone();
                            let orchestrator = self.orchestrator.clone();

                            tokio::spawn(async move {
                                let _guard = lock.lock().await;
                                Self::dispatch(channel_msg, conv_id, adapter, orchestrator)
                                    .await;
                            });
                        }
                    }
                }
            }
        }

        self.adapter.stop().await?;
        Ok(())
    }

    /// Default (standalone) shutdown signal: Ctrl-C or SIGTERM.
    async fn default_shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            () = ctrl_c => {}
            () = terminate => {}
        }
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use lru::LruCache;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use super::ChannelRunner;

    /// `resolve_conv_id` returns the same UUID for the same key.
    #[tokio::test]
    async fn resolve_conv_id_stable() {
        let convs = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));
        let id1 = ChannelRunner::resolve_conv_id(&convs, "chan/thread").await;
        let id2 = ChannelRunner::resolve_conv_id(&convs, "chan/thread").await;
        assert_eq!(id1, id2, "same key must resolve to same UUID");
    }

    /// `resolve_conv_id` returns different UUIDs for different keys.
    #[tokio::test]
    async fn resolve_conv_id_different_keys() {
        let convs = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));
        let id1 = ChannelRunner::resolve_conv_id(&convs, "chan/thread1").await;
        let id2 = ChannelRunner::resolve_conv_id(&convs, "chan/thread2").await;
        assert_ne!(id1, id2, "different keys must produce different UUIDs");
    }

    /// `get_conv_lock` returns the same `Arc<Mutex>` for the same UUID.
    #[tokio::test]
    async fn get_conv_lock_same_uuid() {
        let locks = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let id = Uuid::new_v4();
        let l1 = ChannelRunner::get_conv_lock(&locks, id).await;
        let l2 = ChannelRunner::get_conv_lock(&locks, id).await;
        assert!(Arc::ptr_eq(&l1, &l2), "same UUID must share the same lock");
    }

    /// `get_conv_lock` returns different locks for different UUIDs.
    #[tokio::test]
    async fn get_conv_lock_different_uuids() {
        let locks = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let l1 = ChannelRunner::get_conv_lock(&locks, Uuid::new_v4()).await;
        let l2 = ChannelRunner::get_conv_lock(&locks, Uuid::new_v4()).await;
        assert!(
            !Arc::ptr_eq(&l1, &l2),
            "different UUIDs must have different locks"
        );
    }

    /// Per-conversation lock serialises concurrent turns.
    #[tokio::test]
    async fn conv_lock_serialises_turns() {
        let locks = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let id = Uuid::new_v4();
        let lock = ChannelRunner::get_conv_lock(&locks, id).await;

        let counter = Arc::new(Mutex::new(0u32));

        let lock1 = lock.clone();
        let c1 = counter.clone();
        let t1 = tokio::spawn(async move {
            let _guard = lock1.lock().await;
            let mut c = c1.lock().await;
            *c += 1;
        });

        let c2 = counter.clone();
        let t2 = tokio::spawn(async move {
            let _guard = lock.lock().await;
            let mut c = c2.lock().await;
            *c += 1;
        });

        let _ = tokio::join!(t1, t2);
        assert_eq!(*counter.lock().await, 2, "both turns must complete");
    }
}
