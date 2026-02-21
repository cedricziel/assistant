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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_store_path_is_used_verbatim() {
        let cfg = SignalConfig {
            store_path: Some("/tmp/my-signal-store".to_string()),
            ..Default::default()
        };
        assert_eq!(
            cfg.resolved_store_path(),
            std::path::PathBuf::from("/tmp/my-signal-store")
        );
    }

    #[test]
    fn default_store_path_falls_back_to_home() {
        let cfg = SignalConfig::default();
        let path = cfg.resolved_store_path();
        // dirs::home_dir() may return None in CI/container environments.
        // Accept both the home-dir path and the last-resort fallback.
        assert!(
            path.ends_with(".assistant/signal-store") || path.ends_with(".signal-store"),
            "unexpected path: {path:?}"
        );
    }

    #[test]
    fn allowed_senders_empty_by_default() {
        let cfg = SignalConfig::default();
        assert!(cfg.allowed_senders.is_empty());
    }

    #[test]
    fn signal_config_roundtrips_toml() {
        let toml_str = r#"
            phone_number = "+14155550123"
            allowed_senders = ["uuid-a", "uuid-b"]
            store_path = "/var/lib/signal"
        "#;
        let cfg: SignalConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(cfg.phone_number.as_deref(), Some("+14155550123"));
        assert_eq!(cfg.allowed_senders, ["uuid-a", "uuid-b"]);
        assert_eq!(
            cfg.resolved_store_path(),
            std::path::PathBuf::from("/var/lib/signal")
        );
    }

    #[test]
    fn assistant_config_with_signal_section_roundtrips() {
        let toml_str = r#"
            [signal]
            phone_number = "+14155550123"
            allowed_senders = ["uuid-x"]
        "#;
        let cfg: assistant_core::AssistantConfig = toml::from_str(toml_str).expect("parse");
        let sig = cfg.signal.expect("signal section present");
        assert_eq!(sig.phone_number.as_deref(), Some("+14155550123"));
        assert_eq!(sig.allowed_senders, ["uuid-x"]);
    }

    #[test]
    fn assistant_config_without_signal_section_is_none() {
        let cfg: assistant_core::AssistantConfig = toml::from_str("").expect("parse");
        assert!(cfg.signal.is_none());
    }
}
