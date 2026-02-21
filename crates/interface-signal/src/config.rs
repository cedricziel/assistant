//! Signal interface configuration.
//!
//! [`SignalConfig`] is defined in `assistant-core` so it can be embedded in
//! [`AssistantConfig`][assistant_core::AssistantConfig].  This module
//! re-exports it and adds runtime helpers (e.g. `resolved_store_path`) that
//! depend on the `dirs` crate, which is not a dependency of `assistant-core`.

use std::path::PathBuf;

pub use assistant_core::SignalConfig;

/// Extension methods for [`SignalConfig`] that require the `dirs` crate.
pub trait SignalConfigExt {
    /// Resolve the store path.
    ///
    /// Falls back to `~/.assistant/signal-store` if no path is configured,
    /// and to `.signal-store` relative to the working directory as a last
    /// resort.
    fn resolved_store_path(&self) -> PathBuf;
}

impl SignalConfigExt for SignalConfig {
    fn resolved_store_path(&self) -> PathBuf {
        self.store_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".assistant").join("signal-store")))
            .unwrap_or_else(|| PathBuf::from(".signal-store"))
    }
}
