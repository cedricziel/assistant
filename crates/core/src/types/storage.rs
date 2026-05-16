//! Storage and message-bus configuration.

use serde::{Deserialize, Serialize};

/// Storage configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: Option<String>,
}

/// Message bus backend kind.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BusKind {
    /// SQLite-backed bus (default) — uses the same pool as storage.
    #[default]
    Sqlite,
    /// NATS JetStream-backed bus — eliminates SQLite write-lock contention.
    Nats,
}

/// Message bus configuration.
///
/// Controls which bus backend is used. Add a `[bus]` section to
/// `config.toml`:
///
/// ```toml
/// [bus]
/// kind = "nats"
/// nats_url = "nats://localhost:4222"
/// ```
///
/// The `nats_url` field falls back to the `NATS_URL` environment variable.
///
/// ## Authentication
///
/// Multiple auth methods are supported (in priority order):
///
/// 1. **Credentials file** (`credentials_file`) — `.creds` file with JWT + NKey
/// 2. **Token** (`token` or `NATS_TOKEN` env var)
/// 3. **Username/password** (`username`/`password` or `NATS_USER`/`NATS_PASSWORD` env vars)
///
/// If none are set, the connection is unauthenticated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BusConfig {
    /// Which bus backend to use: `"sqlite"` (default) or `"nats"`.
    #[serde(default)]
    pub kind: BusKind,
    /// NATS server URL (e.g. `nats://localhost:4222`).
    /// Falls back to the `NATS_URL` environment variable when not set.
    pub nats_url: Option<String>,
    /// Username for NATS authentication.
    /// Falls back to the `NATS_USER` environment variable.
    pub username: Option<String>,
    /// Password for NATS authentication.
    /// Falls back to the `NATS_PASSWORD` environment variable.
    pub password: Option<String>,
    /// Authentication token for NATS.
    /// Falls back to the `NATS_TOKEN` environment variable.
    pub token: Option<String>,
    /// Path to a `.creds` file for NATS JWT + NKey authentication.
    /// Falls back to the `NATS_CREDENTIALS_FILE` environment variable.
    pub credentials_file: Option<String>,
}
