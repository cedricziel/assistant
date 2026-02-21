//! Signal interface configuration.

use serde::{Deserialize, Serialize};

/// Configuration for the Signal messenger interface.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignalConfig {
    /// The phone number registered with Signal (e.g. "+14155550123").
    pub phone_number: Option<String>,

    /// If non-empty, only these sender numbers are allowed to interact with
    /// the bot.  An empty list means all contacts are accepted.
    #[serde(default)]
    pub allowed_senders: Vec<String>,

    /// Path where presage stores its Signal state (defaults to
    /// `~/.assistant/signal-store`).
    pub store_path: Option<String>,
}

impl SignalConfig {
    /// Resolve the store path, falling back to `~/.assistant/signal-store`.
    pub fn resolved_store_path(&self) -> std::path::PathBuf {
        self.store_path
            .as_ref()
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".assistant").join("signal-store")))
            .unwrap_or_else(|| std::path::PathBuf::from(".signal-store"))
    }
}
