//! Configuration for the messenger interface adapters: Signal, Slack,
//! Mattermost, Matrix, and Nextcloud Talk.

use serde::{Deserialize, Serialize};

// ── Signal ────────────────────────────────────────────────────────────────────

/// Configuration for the Signal messenger interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    /// The phone number registered with Signal (e.g. `"+14155550123"`).
    pub phone_number: Option<String>,

    /// If non-empty, only messages from these sender identifiers are
    /// dispatched to the orchestrator.  An empty list accepts all contacts.
    #[serde(default)]
    pub allowed_senders: Vec<String>,

    /// Base URL of the signal-cli-rest-api daemon.
    /// Defaults to `http://localhost:8080` if not set.
    pub api_url: Option<String>,

    /// HTTP Basic Auth username for signal-cli-rest-api (optional).
    pub api_user: Option<String>,

    /// HTTP Basic Auth password for signal-cli-rest-api (optional).
    pub api_password: Option<String>,
}

// ── Slack ─────────────────────────────────────────────────────────────────────

/// Controls which messages the Slack bot reacts to.
///
/// - `Mention` (default) — respond only when `@`-mentioned, in DMs, or in
///   threads the bot is already participating in.
/// - `All` — respond to every message in allowed channels (previous default).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SlackListenMode {
    /// Respond only to `@`-mentions, DMs, and thread replies.
    #[default]
    Mention,
    /// Respond to every message in allowed channels.
    All,
}

/// Configuration for the Slack interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlackConfig {
    /// Bot OAuth token (`xoxb-…`) for sending messages via the Web API.
    pub bot_token: Option<String>,
    /// App-level token (`xapp-…`) for Socket Mode connections.
    pub app_token: Option<String>,
    /// If non-empty, only dispatch messages from these channel IDs.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// If non-empty, only dispatch messages from these Slack user IDs.
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Which messages the bot should react to.
    #[serde(default)]
    pub mode: SlackListenMode,
}

// ── Mattermost ────────────────────────────────────────────────────────────────

/// Configuration for the Mattermost interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MattermostConfig {
    /// Base URL of the Mattermost server (e.g. `"https://mattermost.example.com"`).
    pub server_url: Option<String>,
    /// Personal access token or bot token for authentication.
    pub token: Option<String>,
    /// If non-empty, only dispatch messages from these channel IDs.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// If non-empty, only dispatch messages from these Mattermost user IDs.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

// ── Matrix ────────────────────────────────────────────────────────────────────

/// Configuration for the Matrix interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatrixConfig {
    /// Base URL of the Matrix homeserver (e.g. `"https://matrix.example.com"`).
    pub homeserver_url: Option<String>,
    /// Full Matrix user ID of the bot account (e.g. `"@assistant:example.com"`).
    pub username: Option<String>,
    /// Bot account password (used for initial login; session persisted to disk).
    /// Prefer `access_token` for production deployments.
    pub password: Option<String>,
    /// Pre-issued Matrix access token (skips password login on every start).
    pub access_token: Option<String>,
    /// Device ID for session restoration. Auto-generated on first login if omitted.
    pub device_id: Option<String>,
    /// Path for the matrix-sdk SQLite state store.
    /// Defaults to `~/.assistant/matrix-state/` at runtime.
    pub state_store_path: Option<String>,
    /// If non-empty, only dispatch messages from these Matrix room IDs.
    /// An empty list accepts all rooms.
    #[serde(default)]
    pub allowed_rooms: Vec<String>,
    /// If non-empty, only dispatch messages from these Matrix user IDs.
    /// An empty list accepts all users.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

// ── Nextcloud Talk ────────────────────────────────────────────────────────────

/// Configuration for the Nextcloud Talk interface.
///
/// The bot is installed on the Nextcloud server via
/// `occ talk:bot:install` which sets up the webhook URL and a shared secret.
/// The assistant runs an HTTP server that receives incoming webhook callbacks
/// and replies via the Nextcloud Talk Bot REST API.
///
/// ```toml
/// [nextcloud]
/// server_url = "https://nextcloud.example.com"
/// secret = "shared-secret-from-occ-install"
/// listen_addr = "0.0.0.0:8080"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextcloudConfig {
    /// Base URL of the Nextcloud server (e.g. `"https://nextcloud.example.com"`).
    pub server_url: Option<String>,
    /// Shared secret configured when registering the bot via `occ talk:bot:install`.
    /// Also checked via `NEXTCLOUD_TALK_SECRET` env var.
    pub secret: Option<String>,
    /// Socket address the webhook HTTP server listens on (default: `"0.0.0.0:8080"`).
    #[serde(default = "default_nextcloud_listen_addr")]
    pub listen_addr: String,
    /// If non-empty, only dispatch messages from these conversation tokens.
    #[serde(default)]
    pub allowed_channels: Vec<String>,
    /// If non-empty, only dispatch messages from these Nextcloud user IDs.
    #[serde(default)]
    pub allowed_users: Vec<String>,
}

impl Default for NextcloudConfig {
    fn default() -> Self {
        Self {
            server_url: None,
            secret: None,
            listen_addr: default_nextcloud_listen_addr(),
            allowed_channels: Vec::new(),
            allowed_users: Vec::new(),
        }
    }
}

fn default_nextcloud_listen_addr() -> String {
    "0.0.0.0:8080".to_string()
}
