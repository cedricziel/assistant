//! Configuration helpers for the Nextcloud Talk interface.
//!
//! The config struct itself (`NextcloudConfig`) lives in `assistant-core` so
//! it can be embedded in `AssistantConfig`.  This module adds an extension
//! trait with runtime helpers (env-var fallbacks).

pub use assistant_core::types::channels::NextcloudConfig;

/// Extension methods for [`NextcloudConfig`] that provide env-var fallbacks.
pub trait NextcloudConfigExt {
    /// Returns the Nextcloud server URL, falling back to the
    /// `NEXTCLOUD_SERVER_URL` environment variable.
    ///
    /// Returns `None` if the value is blank or whitespace-only.
    fn resolved_server_url(&self) -> Option<String>;

    /// Returns the shared secret, falling back to the
    /// `NEXTCLOUD_TALK_SECRET` environment variable.
    ///
    /// Returns `None` if the value is blank or whitespace-only.
    fn resolved_secret(&self) -> Option<String>;
}

/// Filter out blank/whitespace-only strings so empty config values and
/// empty env vars are treated as unset.
fn non_blank(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}

impl NextcloudConfigExt for NextcloudConfig {
    fn resolved_server_url(&self) -> Option<String> {
        non_blank(self.server_url.clone())
            .or_else(|| non_blank(std::env::var("NEXTCLOUD_SERVER_URL").ok()))
    }

    fn resolved_secret(&self) -> Option<String> {
        non_blank(self.secret.clone())
            .or_else(|| non_blank(std::env::var("NEXTCLOUD_TALK_SECRET").ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_with_explicit_values() {
        let cfg = NextcloudConfig {
            server_url: Some("https://nc.example.com".to_string()),
            secret: Some("s3cret".to_string()),
            ..Default::default()
        };
        assert_eq!(
            cfg.resolved_server_url().as_deref(),
            Some("https://nc.example.com")
        );
        assert_eq!(cfg.resolved_secret().as_deref(), Some("s3cret"));
    }

    #[test]
    fn config_defaults() {
        let toml_str = r#"
            server_url = "https://nc.local"
            secret = "test"
        "#;
        let cfg: NextcloudConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.listen_addr, "0.0.0.0:8080");
        assert!(cfg.allowed_channels.is_empty());
        assert!(cfg.allowed_users.is_empty());
    }

    #[test]
    fn blank_values_treated_as_none() {
        let cfg = NextcloudConfig {
            server_url: Some("".to_string()),
            secret: Some("   ".to_string()),
            ..Default::default()
        };
        assert!(cfg.resolved_server_url().is_none());
        assert!(cfg.resolved_secret().is_none());
    }

    #[test]
    fn default_listen_addr_via_default_trait() {
        let cfg = NextcloudConfig::default();
        assert_eq!(cfg.listen_addr, "0.0.0.0:8080");
    }
}
