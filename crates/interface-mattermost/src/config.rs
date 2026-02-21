//! Mattermost interface configuration.
//!
//! [`MattermostConfig`] is defined in `assistant-core` so it can be embedded
//! in [`AssistantConfig`][assistant_core::AssistantConfig].  This module
//! re-exports it and adds runtime helpers that depend on the `dirs` crate,
//! which is not a dependency of `assistant-core`.

pub use assistant_core::MattermostConfig;

/// Extension methods for [`MattermostConfig`] that require the `dirs` crate.
pub trait MattermostConfigExt {
    /// Return the server URL, falling back to the `MATTERMOST_SERVER_URL`
    /// environment variable if not configured in `config.toml`.
    fn resolved_server_url(&self) -> Option<String>;

    /// Return the access token, falling back to the `MATTERMOST_TOKEN`
    /// environment variable if not configured in `config.toml`.
    fn resolved_token(&self) -> Option<String>;
}

impl MattermostConfigExt for MattermostConfig {
    fn resolved_server_url(&self) -> Option<String> {
        self.server_url
            .clone()
            .or_else(|| std::env::var("MATTERMOST_SERVER_URL").ok())
    }

    fn resolved_token(&self) -> Option<String> {
        self.token
            .clone()
            .or_else(|| std::env::var("MATTERMOST_TOKEN").ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_values_are_used_verbatim() {
        let cfg = MattermostConfig {
            server_url: Some("https://mm.example.com".to_string()),
            token: Some("tok-abc".to_string()),
            ..Default::default()
        };
        assert_eq!(
            cfg.resolved_server_url().as_deref(),
            Some("https://mm.example.com")
        );
        assert_eq!(cfg.resolved_token().as_deref(), Some("tok-abc"));
    }

    #[test]
    fn allowed_channels_empty_by_default() {
        let cfg = MattermostConfig::default();
        assert!(cfg.allowed_channels.is_empty());
    }

    #[test]
    fn allowed_users_empty_by_default() {
        let cfg = MattermostConfig::default();
        assert!(cfg.allowed_users.is_empty());
    }

    #[test]
    fn mattermost_config_roundtrips_toml() {
        let toml_str = r#"
            server_url = "https://mm.example.com"
            token = "tok-xyz"
            allowed_channels = ["town-square", "bot-test"]
            allowed_users = ["alice"]
        "#;
        let cfg: MattermostConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(cfg.server_url.as_deref(), Some("https://mm.example.com"));
        assert_eq!(cfg.token.as_deref(), Some("tok-xyz"));
        assert_eq!(cfg.allowed_channels, ["town-square", "bot-test"]);
        assert_eq!(cfg.allowed_users, ["alice"]);
    }

    #[test]
    fn assistant_config_with_mattermost_section_roundtrips() {
        let toml_str = r#"
            [mattermost]
            server_url = "https://mm.example.com"
            token = "tok-abc"
        "#;
        let cfg: assistant_core::AssistantConfig = toml::from_str(toml_str).expect("parse");
        let mm = cfg.mattermost.expect("mattermost section present");
        assert_eq!(mm.server_url.as_deref(), Some("https://mm.example.com"));
        assert_eq!(mm.token.as_deref(), Some("tok-abc"));
    }

    #[test]
    fn assistant_config_without_mattermost_section_is_none() {
        let cfg: assistant_core::AssistantConfig = toml::from_str("").expect("parse");
        assert!(cfg.mattermost.is_none());
    }
}
