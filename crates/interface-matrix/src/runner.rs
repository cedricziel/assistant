//! Matrix interface runner.
//!
//! Authenticates with a Matrix homeserver and enters the matrix-sdk sync loop.
//! Each incoming room message is dispatched to the [`Orchestrator`] and the
//! reply is posted back into the same room.
//!
//! # Authentication
//!
//! Two paths are supported (tried in order):
//!
//! 1. **Access token** — set `access_token` (and optionally `device_id`) in
//!    `[matrix]` config or via `MATRIX_ACCESS_TOKEN`.
//! 2. **Password login** — set `username` + `password`.  The SDK persists the
//!    resulting session to the SQLite state store, so subsequent restarts skip
//!    the password exchange.
//!
//! # Conversation keying
//!
//! Each Matrix room maps to a single LRU-bounded [`Uuid`] conversation.
//! Room IDs are stable identifiers, so the mapping is consistent across
//! reconnects within a single process run.

use std::num::NonZeroUsize;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::Interface;
use assistant_runtime::Orchestrator;
use lru::LruCache;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::events::room::member::StrippedRoomMemberEvent;
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
use matrix_sdk::{Client, Room, RoomState};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::{MatrixConfig, MatrixConfigExt};

// ── MatrixInterface ───────────────────────────────────────────────────────────

/// The Matrix interface handle.
///
/// Call [`MatrixInterface::run`] to start the sync loop.
pub struct MatrixInterface {
    config: MatrixConfig,
    orchestrator: Arc<Orchestrator>,
}

impl MatrixInterface {
    /// Create a new interface handle.
    pub fn new(config: MatrixConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Matrix sync loop.
    ///
    /// Logs in (or restores a session), registers event handlers, then enters
    /// `client.sync()`.  Exits cleanly on SIGINT / SIGTERM.
    pub async fn run(&self) -> Result<()> {
        let homeserver_url = self.config.resolved_homeserver_url().ok_or_else(|| {
            anyhow::anyhow!(
                "No Matrix homeserver URL configured. \
                     Set homeserver_url in [matrix] config or MATRIX_HOMESERVER_URL env var."
            )
        })?;

        // Determine state store path.
        let state_store_path = self
            .config
            .resolved_state_store_path()
            .or_else(|| {
                dirs::home_dir().map(|h| {
                    h.join(".assistant")
                        .join("matrix-state")
                        .to_string_lossy()
                        .into_owned()
                })
            })
            .unwrap_or_else(|| ".assistant/matrix-state".to_string());

        tokio::fs::create_dir_all(&state_store_path).await?;

        info!(homeserver = %homeserver_url, state_store = %state_store_path, "Building Matrix client");

        let client = Client::builder()
            .homeserver_url(&homeserver_url)
            .sqlite_store(&state_store_path, None)
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to build Matrix client: {e}"))?;

        // ── Authentication ────────────────────────────────────────────────────
        if let Some(access_token) = self.config.resolved_access_token() {
            // Attempt to restore an existing session from the state store first.
            // If that fails (no session yet), set the access token directly.
            if client.matrix_auth().logged_in() {
                info!("Matrix session already restored from state store");
            } else {
                info!("Logging in with access token");
                let device_id = self.config.device_id.as_deref();

                use matrix_sdk::authentication::matrix::{MatrixSession, MatrixSessionTokens};
                use matrix_sdk::ruma::{DeviceId, OwnedDeviceId, OwnedUserId, UserId};
                use matrix_sdk::SessionMeta;

                let username = self.config.resolved_username().ok_or_else(|| {
                    anyhow::anyhow!(
                        "access_token is set but username is missing. \
                         Set username in [matrix] config or MATRIX_USERNAME env var."
                    )
                })?;

                let user_id = <&UserId>::try_from(username.as_str())
                    .map(OwnedUserId::from)
                    .map_err(|e| anyhow::anyhow!("Invalid Matrix user ID '{username}': {e}"))?;

                let device_id: OwnedDeviceId = device_id
                    .map(|d| OwnedDeviceId::from(<&DeviceId>::from(d)))
                    .unwrap_or_else(DeviceId::new);

                let session = MatrixSession {
                    meta: SessionMeta { user_id, device_id },
                    tokens: MatrixSessionTokens {
                        access_token,
                        refresh_token: None,
                    },
                };

                client
                    .matrix_auth()
                    .restore_session(session)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to restore Matrix session: {e}"))?;

                info!(user_id = ?client.user_id(), "Matrix session restored via access token");
            }
        } else {
            let username = self.config.resolved_username().ok_or_else(|| {
                anyhow::anyhow!(
                    "No Matrix credentials configured. Set access_token or username+password \
                     in [matrix] config (or MATRIX_ACCESS_TOKEN / MATRIX_USERNAME + MATRIX_PASSWORD)."
                )
            })?;
            let password = self.config.resolved_password().ok_or_else(|| {
                anyhow::anyhow!(
                    "username is set but password is missing. \
                     Set password in [matrix] config or MATRIX_PASSWORD env var."
                )
            })?;

            if !client.matrix_auth().logged_in() {
                info!(username = %username, "Logging in with username/password");
                client
                    .matrix_auth()
                    .login_username(&username, &password)
                    .initial_device_display_name("assistant-matrix-bot")
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("Matrix login failed: {e}"))?;
                info!(user_id = ?client.user_id(), "Matrix login successful");
            } else {
                info!("Matrix session already restored from state store");
            }
        }

        let bot_user_id = client.user_id().map(|u| u.to_string()).unwrap_or_default();

        info!(bot_user_id = %bot_user_id, "Matrix bot ready");

        // ── Shared conversation state ─────────────────────────────────────────

        // LRU map: room_id (String) → conversation UUID
        let conversations: Arc<Mutex<LruCache<String, Uuid>>> = Arc::new(Mutex::new(
            LruCache::new(NonZeroUsize::new(10_000).expect("capacity is non-zero")),
        ));

        // ── Graceful shutdown ─────────────────────────────────────────────────
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                match signal(SignalKind::terminate()) {
                    Ok(mut sigterm) => {
                        tokio::select! {
                            _ = tokio::signal::ctrl_c() => {}
                            _ = sigterm.recv() => {}
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to install SIGTERM handler; only Ctrl+C shutdown will work");
                        let _ = tokio::signal::ctrl_c().await;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
            info!("Shutdown signal received, stopping Matrix interface");
            let _ = shutdown_tx.send(true);
        });

        // ── BOOT.md hook ──────────────────────────────────────────────────────
        {
            let boot_conversation_id = Uuid::new_v4();
            match self
                .orchestrator
                .run_boot(boot_conversation_id, Interface::Matrix)
                .await
            {
                Ok(true) => info!("BOOT.md startup hook executed"),
                Ok(false) => {}
                Err(e) => warn!(error = %e, "BOOT.md startup hook failed"),
            }
        }

        // ── Event handlers ────────────────────────────────────────────────────

        // Auto-accept room invitations.
        client.add_event_handler(
            |event: StrippedRoomMemberEvent, room: Room, client: Client| async move {
                let Some(my_user_id) = client.user_id() else {
                    return;
                };
                if event.state_key != *my_user_id {
                    return;
                }
                use matrix_sdk::ruma::events::room::member::MembershipState;
                if event.content.membership != MembershipState::Invite {
                    return;
                }
                info!(room = %room.room_id(), "Auto-accepting room invitation");
                if let Err(e) = room.join().await {
                    warn!(error = %e, room = %room.room_id(), "Failed to accept room invitation");
                }
            },
        );

        // Dispatch incoming text messages to the orchestrator.
        let orchestrator = self.orchestrator.clone();
        let config = self.config.clone();
        let conversations_clone = conversations.clone();

        client.add_event_handler(move |event: OriginalSyncRoomMessageEvent, room: Room| {
            let orchestrator = orchestrator.clone();
            let config = config.clone();
            let bot_uid = bot_user_id.clone();
            let conversations = conversations_clone.clone();

            async move {
                // Only handle joined rooms.
                if room.state() != RoomState::Joined {
                    return;
                }

                // Only handle text messages.
                let MessageType::Text(text_content) = &event.content.msgtype else {
                    return;
                };

                let text = text_content.body.clone();
                let sender = event.sender.to_string();
                let room_id = room.room_id().to_string();

                // Ignore the bot's own messages to prevent infinite loops.
                if sender == bot_uid {
                    debug!(sender = %sender, "Ignoring self-message");
                    return;
                }

                // Room allowlist check.
                if !config.allowed_rooms.is_empty() && !config.allowed_rooms.contains(&room_id) {
                    warn!(room = %room_id, "Ignoring message from non-allowlisted room");
                    return;
                }

                // User allowlist check.
                if !config.allowed_users.is_empty() && !config.allowed_users.contains(&sender) {
                    warn!(user = %sender, "Ignoring message from non-allowlisted user");
                    return;
                }

                info!(
                    room = %room_id,
                    sender = %sender,
                    text_len = text.len(),
                    "Incoming Matrix message"
                );

                // Look up or create a conversation for this room.
                let conversation_id = {
                    let mut map = conversations.lock().await;
                    if let Some(&id) = map.get(&room_id) {
                        id
                    } else {
                        let id = Uuid::new_v4();
                        map.put(room_id.clone(), id);
                        id
                    }
                };

                let turn_result = orchestrator
                    .submit_turn(&text, conversation_id, Interface::Matrix, None)
                    .await;

                match turn_result {
                    Err(e) => {
                        tracing::error!(error = %e, room = %room_id, "Orchestrator error");
                        let _ = room
                            .send(RoomMessageEventContent::text_plain(
                                "Sorry, something went wrong processing your message.",
                            ))
                            .await;
                    }
                    Ok(_) => {
                        info!(room = %room_id, "submit_turn ← ok");
                    }
                }
            }
        });

        // ── Sync loop ─────────────────────────────────────────────────────────
        let mut backoff = std::time::Duration::from_secs(1);

        loop {
            if *shutdown_rx.borrow() {
                break;
            }

            info!("Starting Matrix sync");

            let sync_settings = SyncSettings::default();
            let result = tokio::select! {
                r = client.sync(sync_settings) => r,
                _ = shutdown_rx.changed() => {
                    info!("Shutdown during sync, exiting");
                    return Ok(());
                }
            };

            match result {
                Ok(()) => {
                    info!("Matrix sync ended cleanly, reconnecting");
                    backoff = std::time::Duration::from_secs(1);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        delay_secs = backoff.as_secs(),
                        "Matrix sync error, retrying"
                    );
                    backoff = (backoff * 2).min(std::time::Duration::from_secs(60));
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                _ = shutdown_rx.changed() => {
                    info!("Shutdown during backoff, exiting");
                    return Ok(());
                }
            }
        }

        info!("Matrix interface stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::MatrixConfig;

    // ── Allowlist logic unit tests ────────────────────────────────────────────

    #[test]
    fn allowlist_room_empty_accepts_all() {
        let cfg = MatrixConfig {
            allowed_rooms: vec![],
            ..Default::default()
        };
        let room_id = "!abc:example.com".to_string();
        let blocked = !cfg.allowed_rooms.is_empty() && !cfg.allowed_rooms.contains(&room_id);
        assert!(!blocked, "empty allowed_rooms should accept all rooms");
    }

    #[test]
    fn allowlist_room_non_empty_blocks_unknown() {
        let cfg = MatrixConfig {
            allowed_rooms: vec!["!known:example.com".to_string()],
            ..Default::default()
        };
        let unknown = "!other:example.com".to_string();
        let blocked = !cfg.allowed_rooms.is_empty() && !cfg.allowed_rooms.contains(&unknown);
        assert!(
            blocked,
            "non-empty allowed_rooms should block unknown rooms"
        );
    }

    #[test]
    fn allowlist_room_non_empty_passes_known() {
        let cfg = MatrixConfig {
            allowed_rooms: vec!["!known:example.com".to_string()],
            ..Default::default()
        };
        let known = "!known:example.com".to_string();
        let blocked = !cfg.allowed_rooms.is_empty() && !cfg.allowed_rooms.contains(&known);
        assert!(!blocked, "known room should not be blocked");
    }

    #[test]
    fn allowlist_user_empty_accepts_all() {
        let cfg = MatrixConfig {
            allowed_users: vec![],
            ..Default::default()
        };
        let user = "@alice:example.com".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&user);
        assert!(!blocked, "empty allowed_users should accept all users");
    }

    #[test]
    fn allowlist_user_non_empty_blocks_unknown() {
        let cfg = MatrixConfig {
            allowed_users: vec!["@alice:example.com".to_string()],
            ..Default::default()
        };
        let unknown = "@eve:example.com".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&unknown);
        assert!(
            blocked,
            "non-empty allowed_users should block unknown users"
        );
    }

    #[test]
    fn allowlist_user_non_empty_passes_known() {
        let cfg = MatrixConfig {
            allowed_users: vec!["@alice:example.com".to_string()],
            ..Default::default()
        };
        let known = "@alice:example.com".to_string();
        let blocked = !cfg.allowed_users.is_empty() && !cfg.allowed_users.contains(&known);
        assert!(!blocked, "known user should not be blocked");
    }

    // ── LRU conversation isolation test ──────────────────────────────────────

    #[tokio::test]
    async fn lru_rooms_get_independent_conversation_ids() {
        use std::num::NonZeroUsize;
        use std::sync::Arc;

        use lru::LruCache;
        use tokio::sync::Mutex;
        use uuid::Uuid;

        let conversations: Arc<Mutex<LruCache<String, Uuid>>> = Arc::new(Mutex::new(
            LruCache::new(NonZeroUsize::new(10).expect("non-zero")),
        ));

        let room_a = "!room_a:example.com".to_string();
        let room_b = "!room_b:example.com".to_string();

        let id_a1 = {
            let mut map = conversations.lock().await;
            let id = Uuid::new_v4();
            map.put(room_a.clone(), id);
            id
        };
        let id_b1 = {
            let mut map = conversations.lock().await;
            let id = Uuid::new_v4();
            map.put(room_b.clone(), id);
            id
        };

        // Room A keeps its ID.
        let id_a2 = {
            let mut map = conversations.lock().await;
            *map.get(&room_a).expect("room_a present")
        };
        assert_eq!(id_a1, id_a2, "room_a conversation ID should be stable");

        // Rooms A and B have different IDs.
        assert_ne!(id_a1, id_b1, "separate rooms get separate conversation IDs");
    }
}
