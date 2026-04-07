//! Slack interface runner.
//!
//! Drives the [`SlackAdapter`] stream → orchestrator dispatch loop.
//! Each inbound [`ChannelMessage`] is dispatched to the orchestrator with
//! per-conversation serialisation (one turn at a time per thread).

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ChannelAdapter, ChannelContent, ChannelMessage, Interface, SlackConfig};
use assistant_runtime::{InterfaceRunner, Orchestrator};
use assistant_storage::StorageLayer;
use assistant_transcription::TranscriptionProvider;
use futures::StreamExt;
use lru::LruCache;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::adapter::SlackAdapter;
use crate::config::SlackConfigExt;
use crate::skills::{
    SlackDeleteMessageSkill, SlackGetHistorySkill, SlackListChannelsSkill, SlackLookupUserSkill,
    SlackPostSkill, SlackReactSkill, SlackSendDmSkill, SlackUpdateMessageSkill,
};
use crate::tools::build_slack_tools;

/// Slack interface runner.  Connects via Socket Mode and dispatches messages.
pub struct SlackInterface {
    config: SlackConfig,
    orchestrator: Arc<Orchestrator>,
    storage: Arc<StorageLayer>,
    transcription: Option<Arc<dyn TranscriptionProvider>>,
    transcription_language: Option<String>,
}

impl SlackInterface {
    pub fn new(
        config: SlackConfig,
        orchestrator: Arc<Orchestrator>,
        storage: Arc<StorageLayer>,
    ) -> Self {
        Self {
            config,
            orchestrator,
            storage,
            transcription: None,
            transcription_language: None,
        }
    }

    /// Enable automatic audio transcription for voice messages.
    pub fn with_transcription(
        mut self,
        provider: Arc<dyn TranscriptionProvider>,
        language: Option<String>,
    ) -> Self {
        self.transcription = Some(provider);
        self.transcription_language = language;
        self
    }

    /// Return ambient tools contributed by this interface.
    pub fn ambient_tools(&self) -> Vec<std::sync::Arc<dyn assistant_core::ToolHandler>> {
        let Some(bot_token) = self.config.resolved_bot_token() else {
            return vec![];
        };
        let Some(app_token) = self.config.resolved_app_token() else {
            return vec![];
        };
        let client = match crate::client::SlackApiClient::new(bot_token, app_token) {
            Ok(c) => std::sync::Arc::new(c),
            Err(e) => {
                warn!(error = %e, "slack: failed to build ambient API client");
                return vec![];
            }
        };
        vec![
            std::sync::Arc::new(SlackPostSkill {
                client: client.clone(),
            }) as std::sync::Arc<dyn assistant_core::ToolHandler>,
            std::sync::Arc::new(SlackSendDmSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackListChannelsSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackGetHistorySkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackReactSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackUpdateMessageSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackDeleteMessageSkill {
                client: client.clone(),
            }),
            std::sync::Arc::new(SlackLookupUserSkill { client }),
        ]
    }

    /// Run the Slack interface: connect, receive messages, dispatch to orchestrator.
    pub async fn run(&self) -> Result<()> {
        let adapter = SlackAdapter::new(self.config.clone())?;
        let api_client = adapter.api_client();

        // BOOT.md startup hook (fire-and-forget; ignore errors).
        let boot_id = Uuid::new_v4();
        if let Err(e) = self.orchestrator.run_boot(boot_id, Interface::Slack).await {
            warn!(error = %e, "Slack BOOT hook failed (non-fatal)");
        }

        // Per-conversation serialisation: only one turn runs at a time per UUID.
        let conv_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        // Map (channel_id, thread_ts) → conversation UUID.
        let conversations: Arc<Mutex<LruCache<(String, String), Uuid>>> = Arc::new(Mutex::new(
            LruCache::new(NonZeroUsize::new(10_000).unwrap()),
        ));
        // Dedup set: skip messages we already processed.
        let processed_ts: Arc<Mutex<std::collections::HashSet<String>>> =
            Arc::new(Mutex::new(std::collections::HashSet::new()));

        // Graceful shutdown.
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = sigterm.recv() => {}
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            info!("Slack: shutdown signal received");
            let _ = shutdown_tx.send(true);
        });

        let mut stream = adapter.start().await?;
        info!("Slack: connected and listening");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Slack: shutting down");
                        break;
                    }
                }
                msg = stream.next() => {
                    match msg {
                        None => {
                            warn!("Slack: message stream ended");
                            break;
                        }
                        Some(channel_msg) => {
                            // Dedup by platform_message_id (ts).
                            if let Some(ts) = &channel_msg.platform_message_id {
                                let mut pd = processed_ts.lock().await;
                                if !pd.insert(ts.clone()) {
                                    continue; // already processed
                                }
                                if pd.len() > 500 {
                                    pd.clear();
                                }
                            }

                            let conv_id = get_or_create_conversation(
                                &channel_msg,
                                &conversations,
                                &self.storage,
                                &api_client,
                            ).await;

                            // Acquire per-conversation lock (serialise turns).
                            let lock = {
                                let mut locks = conv_locks.lock().await;
                                locks.entry(conv_id).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
                            };

                            let orchestrator = self.orchestrator.clone();
                            let api = api_client.clone();

                            tokio::spawn(async move {
                                let _guard = lock.lock().await;
                                dispatch_message(channel_msg, conv_id, orchestrator, api).await;
                            });
                        }
                    }
                }
            }
        }

        adapter.stop().await?;
        Ok(())
    }
}

/// Get (or lazily create) a conversation UUID for this `(channel_id, thread_ts)` pair.
///
/// On first touch, fetches the thread history from Slack and seeds it into the
/// conversation store so the LLM has context from prior messages in the thread.
async fn get_or_create_conversation(
    msg: &ChannelMessage,
    conversations: &Arc<Mutex<LruCache<(String, String), Uuid>>>,
    storage: &Arc<StorageLayer>,
    api_client: &Arc<crate::client::SlackApiClient>,
) -> Uuid {
    let channel_id = msg
        .metadata
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let thread_ts = msg
        .thread_id
        .clone()
        .unwrap_or_else(|| msg.platform_message_id.clone().unwrap_or_default());
    let key = (channel_id.clone(), thread_ts.clone());

    let mut convs = conversations.lock().await;
    if let Some(&existing) = convs.get(&key) {
        return existing;
    }

    // New thread — create a UUID and seed history.
    let conv_id = Uuid::new_v4();
    convs.put(key, conv_id);
    drop(convs);

    // Best-effort: fetch thread history for context seeding.
    if !channel_id.is_empty() && !thread_ts.is_empty() {
        match api_client
            .conversations_replies(&channel_id, &thread_ts)
            .await
        {
            Ok(msgs) => {
                let _ = seed_thread_history(conv_id, &msgs, storage).await;
            }
            Err(e) => {
                // Non-fatal: missing history just means the LLM starts fresh.
                tracing::debug!(error = %e, "slack: failed to fetch thread history for seeding");
            }
        }
    }

    conv_id
}

/// Seed thread history messages into the conversation store.
async fn seed_thread_history(
    conv_id: Uuid,
    messages: &[serde_json::Value],
    storage: &Arc<StorageLayer>,
) -> Result<()> {
    use assistant_core::{Message, MessageRole};

    for msg in messages {
        let subtype = msg.get("subtype").and_then(|v| v.as_str());
        // Skip system events (message_changed, etc.)
        if let Some(st) = subtype {
            if st != "file_share" {
                continue;
            }
        }

        let is_bot = msg.get("bot_id").is_some()
            || msg
                .get("display_as_bot")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        let role = if is_bot {
            MessageRole::Assistant
        } else if msg.get("user").is_some() {
            MessageRole::User
        } else {
            continue; // skip unrecognised
        };

        let text = msg
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if text.is_empty() {
            continue;
        }

        let m = Message::new(conv_id, role, text);
        let _ = storage
            .conversation_store_for_agent("default")
            .save_message(&m)
            .await;
    }
    Ok(())
}

/// Dispatch a single `ChannelMessage` to the orchestrator.
async fn dispatch_message(
    msg: ChannelMessage,
    conv_id: Uuid,
    orchestrator: Arc<Orchestrator>,
    api_client: Arc<crate::client::SlackApiClient>,
) {
    let text = match &msg.content {
        ChannelContent::Text(t) => t.clone(),
        _ => return, // non-text messages not yet handled
    };

    let channel_id = msg
        .metadata
        .get("channel_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let thread_ts = msg.thread_id.clone();
    let message_ts = msg
        .platform_message_id
        .clone()
        .unwrap_or_else(|| thread_ts.clone().unwrap_or_default());

    // React with "eyes" to show the bot is processing.
    if !channel_id.is_empty() && !message_ts.is_empty() {
        let _ = api_client
            .add_reaction(&channel_id, &message_ts, "eyes")
            .await;
    }

    let tools = build_slack_tools(
        channel_id.clone(),
        thread_ts.clone(),
        message_ts.clone(),
        api_client.clone(),
    );

    match orchestrator
        .run_turn_with_tools(
            &text,
            conv_id,
            Interface::Slack,
            tools.into_iter().map(Arc::from).collect(),
            None,
            vec![],
        )
        .await
    {
        Ok(_) => {
            // Reply is posted by the `reply` extension tool.
            if !channel_id.is_empty() && !message_ts.is_empty() {
                let _ = api_client
                    .add_reaction(&channel_id, &message_ts, "white_check_mark")
                    .await;
            }
        }
        Err(e) => {
            error!(error = %e, conv_id = %conv_id, "Slack: orchestrator error");
            // Best-effort error reply.
            let _ = api_client
                .post_message(
                    &channel_id,
                    &format!("Sorry, I encountered an error: {e}"),
                    thread_ts.as_deref(),
                )
                .await;
        }
    }
}

#[async_trait::async_trait]
impl InterfaceRunner for SlackInterface {
    async fn run(&self) -> Result<()> {
        SlackInterface::run(self).await
    }
}
