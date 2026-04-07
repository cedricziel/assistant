//! Signal interface configuration.
//!
//! [`SignalConfig`] is defined in `assistant-core` so it can be embedded in
//! [`AssistantConfig`][assistant_core::AssistantConfig].  This module
//! re-exports it and adds runtime helpers (e.g. `resolved_api_url`) that
//! resolve defaults and environment-variable overrides.

pub use assistant_core::SignalConfig;

/// Extension methods for [`SignalConfig`].
pub trait SignalConfigExt {
    /// Resolve the signal-cli-rest-api base URL.
    ///
    /// Returns the configured `api_url` or falls back to
    /// `http://localhost:8080`.
    fn resolved_api_url(&self) -> String;

    /// Return the configured phone number, if any.
    fn resolved_phone_number(&self) -> Option<String>;

    /// Return the HTTP Basic Auth username, if configured.
    fn resolved_api_user(&self) -> Option<String>;

    /// Return the HTTP Basic Auth password, if configured.
    fn resolved_api_password(&self) -> Option<String>;
}

impl SignalConfigExt for SignalConfig {
    fn resolved_api_url(&self) -> String {
        self.api_url
            .clone()
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    }

    fn resolved_phone_number(&self) -> Option<String> {
        self.phone_number.clone()
    }

    fn resolved_api_user(&self) -> Option<String> {
        self.api_user.clone()
    }

    fn resolved_api_password(&self) -> Option<String> {
        self.api_password.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_api_url_is_localhost_8080() {
        let cfg = SignalConfig::default();
        assert_eq!(cfg.resolved_api_url(), "http://localhost:8080");
    }

    #[test]
    fn custom_api_url_is_used_verbatim() {
        let cfg = SignalConfig {
            api_url: Some("http://signal.example.com:8080".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.resolved_api_url(), "http://signal.example.com:8080");
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
            api_url = "http://localhost:8080"
        "#;
        let cfg: SignalConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(cfg.phone_number.as_deref(), Some("+14155550123"));
        assert_eq!(cfg.allowed_senders, ["uuid-a", "uuid-b"]);
        assert_eq!(cfg.resolved_api_url(), "http://localhost:8080");
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
