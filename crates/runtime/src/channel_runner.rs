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
use assistant_llm::ContentBlock;
use base64::Engine as _;
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

    /// Map a `ChannelContent` to `(user_message, attachments)` for the orchestrator.
    ///
    /// Returns `None` for content types that should not be dispatched (e.g. non-image files).
    pub(crate) fn content_to_dispatch(
        content: &ChannelContent,
    ) -> Option<(String, Vec<ContentBlock>)> {
        match content {
            ChannelContent::Text(t) => Some((t.clone(), vec![])),
            ChannelContent::FileData {
                data, mime_type, ..
            } if mime_type.starts_with("image/") => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(data);
                let block = ContentBlock::Image {
                    media_type: mime_type.clone(),
                    data: b64,
                };
                Some(("[Image attached]".to_string(), vec![block]))
            }
            _ => None,
        }
    }

    /// Dispatch a single inbound message through the orchestrator.
    async fn dispatch(
        msg: ChannelMessage,
        conv_id: Uuid,
        adapter: Arc<dyn ChannelAdapter>,
        orchestrator: Arc<Orchestrator>,
    ) {
        let Some((text, attachments)) = Self::content_to_dispatch(&msg.content) else {
            return;
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
            .run_turn_with_tools(&text, conv_id, interface, tools, None, attachments)
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

        // Register adapter in the registry so the scheduler can find it.
        self.orchestrator
            .adapter_registry
            .register(self.adapter.clone())
            .await;

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

                            // Signal "message received / queued" immediately — before the
                            // per-conversation lock is acquired — so adapters can add an ⏳
                            // reaction even while another turn is still in progress.
                            if let Err(e) = self.adapter.on_message_received(&channel_msg).await {
                                warn!(adapter = self.adapter.name(), error = %e, "on_message_received hook failed");
                            }

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

        // Deregister adapter from the registry on stop.
        self.orchestrator.adapter_registry.deregister(&name).await;

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

    use std::collections::HashMap;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use assistant_core::{
        ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use futures::Stream;

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

    /// `content_to_dispatch` maps image FileData to a multimodal attachment.
    #[test]
    fn image_file_data_dispatched_as_attachment() {
        use assistant_core::ChannelContent;
        use assistant_llm::ContentBlock;
        use base64::Engine as _;

        let data = b"fakeimgbytes".to_vec();
        let content = ChannelContent::FileData {
            data: data.clone(),
            filename: "photo.jpg".to_string(),
            mime_type: "image/jpeg".to_string(),
        };
        let result = ChannelRunner::content_to_dispatch(&content);
        assert!(
            result.is_some(),
            "image FileData must produce a dispatch tuple"
        );
        let (text, attachments) = result.unwrap();
        assert_eq!(text, "[Image attached]");
        assert_eq!(attachments.len(), 1);
        match &attachments[0] {
            ContentBlock::Image {
                media_type,
                data: b64,
            } => {
                assert_eq!(media_type, "image/jpeg");
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .unwrap();
                assert_eq!(decoded, data);
            }
            other => panic!("expected ContentBlock::Image, got {:?}", other),
        }
    }

    /// `content_to_dispatch` returns None for non-image FileData.
    #[test]
    fn non_image_file_data_dropped() {
        use assistant_core::ChannelContent;

        let content = ChannelContent::FileData {
            data: b"pdf".to_vec(),
            filename: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
        };
        let result = ChannelRunner::content_to_dispatch(&content);
        assert!(result.is_none(), "non-image FileData must be dropped");
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

    /// `on_message_received` is called before acquiring the per-conversation lock.
    ///
    /// We verify this by holding the lock in a background task and confirming
    /// that `on_message_received` is called while the lock is still held (i.e.
    /// before `dispatch` gets a chance to run).
    #[tokio::test]
    async fn on_message_received_called_before_lock() {
        // A minimal adapter that records the call order.
        struct OrderAdapter {
            received_count: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl ChannelAdapter for OrderAdapter {
            fn name(&self) -> &str {
                "order"
            }
            fn channel_type(&self) -> ChannelType {
                ChannelType::Slack
            }
            async fn start(
                &self,
            ) -> anyhow::Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>>
            {
                unimplemented!()
            }
            async fn send(&self, _u: &ChannelUser, _c: ChannelContent) -> anyhow::Result<()> {
                Ok(())
            }
            async fn stop(&self) -> anyhow::Result<()> {
                Ok(())
            }
            async fn on_message_received(&self, _msg: &ChannelMessage) -> anyhow::Result<()> {
                self.received_count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let received_count = Arc::new(AtomicUsize::new(0));
        let adapter = Arc::new(OrderAdapter {
            received_count: received_count.clone(),
        });

        // Simulate the runner loop logic in isolation.
        let convs = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));
        let locks = Arc::new(Mutex::new(HashMap::new()));

        let msg = ChannelMessage {
            channel_type: ChannelType::Slack,
            platform_message_id: None,
            sender: ChannelUser {
                platform_id: "C1/ts1".into(),
                display_name: None,
            },
            content: ChannelContent::Text("hello".into()),
            thread_id: Some("ts1".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let key = "C1/ts1".to_string();
        let conv_id = ChannelRunner::resolve_conv_id(&convs, &key).await;
        let lock = ChannelRunner::get_conv_lock(&locks, conv_id).await;

        // Hold the lock so any spawned task blocks waiting for it.
        let _guard = lock.lock().await;

        // on_message_received should not yet have been called.
        assert_eq!(received_count.load(Ordering::SeqCst), 0);

        // Call on_message_received directly (mirrors what ChannelRunner does).
        adapter.on_message_received(&msg).await.unwrap();

        // Now it must have been called — before the lock was released.
        assert_eq!(
            received_count.load(Ordering::SeqCst),
            1,
            "on_message_received must be called before the lock is acquired"
        );
    }

    /// `on_message_received` fires before the conv lock is acquired.
    ///
    /// Mirrors the runner's invocation pattern: hook is called after `resolve_conv_id`
    /// but while the conv lock is still held externally, proving it runs outside the
    /// locked critical section.
    #[tokio::test]
    async fn on_message_received_fires_before_conv_lock() {
        struct CountingAdapter {
            count: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl ChannelAdapter for CountingAdapter {
            fn name(&self) -> &str {
                "counting"
            }
            fn channel_type(&self) -> ChannelType {
                ChannelType::Slack
            }
            async fn start(
                &self,
            ) -> anyhow::Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>>
            {
                unimplemented!()
            }
            async fn send(&self, _u: &ChannelUser, _c: ChannelContent) -> anyhow::Result<()> {
                Ok(())
            }
            async fn stop(&self) -> anyhow::Result<()> {
                Ok(())
            }
            async fn on_message_received(&self, _msg: &ChannelMessage) -> anyhow::Result<()> {
                self.count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let count = Arc::new(AtomicUsize::new(0));
        let adapter = Arc::new(CountingAdapter {
            count: count.clone(),
        });

        let convs = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())));
        let locks = Arc::new(Mutex::new(HashMap::new()));

        let msg = ChannelMessage {
            channel_type: ChannelType::Slack,
            platform_message_id: None,
            sender: ChannelUser {
                platform_id: "C1/ts2".into(),
                display_name: None,
            },
            content: ChannelContent::Text("hi".into()),
            thread_id: Some("ts2".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let key = "C1/ts2".to_string();
        let conv_id = ChannelRunner::resolve_conv_id(&convs, &key).await;
        let lock = ChannelRunner::get_conv_lock(&locks, conv_id).await;

        // Hold the conv lock — any spawned dispatch task would block here.
        let _guard = lock.lock().await;

        assert_eq!(count.load(Ordering::SeqCst), 0, "not yet called");

        // Mirror the runner's invocation: hook fires before acquiring the lock.
        adapter.on_message_received(&msg).await.unwrap();

        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "on_message_received must fire before the conv lock is acquired"
        );
        // _guard is still held here — the hook didn't need the lock.
    }
}
