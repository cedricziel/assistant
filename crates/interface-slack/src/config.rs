//! Slack interface configuration.
//!
//! [`SlackConfig`] is defined in `assistant-core` so it can be embedded in
//! [`AssistantConfig`][assistant_core::AssistantConfig].  This module
//! re-exports it and adds runtime helpers that depend on the `dirs` crate,
//! which is not a dependency of `assistant-core`.

pub use assistant_core::SlackConfig;

/// Extension methods for [`SlackConfig`] that require the `dirs` crate.
pub trait SlackConfigExt {
    /// Return the bot token, falling back to the `SLACK_BOT_TOKEN` environment
    /// variable if no token is configured in `config.toml`.
    fn resolved_bot_token(&self) -> Option<String>;

    /// Return the app-level token, falling back to the `SLACK_APP_TOKEN`
    /// environment variable if no token is configured in `config.toml`.
    fn resolved_app_token(&self) -> Option<String>;
}

impl SlackConfigExt for SlackConfig {
    fn resolved_bot_token(&self) -> Option<String> {
        self.bot_token
            .clone()
            .or_else(|| std::env::var("SLACK_BOT_TOKEN").ok())
    }

    fn resolved_app_token(&self) -> Option<String> {
        self.app_token
            .clone()
            .or_else(|| std::env::var("SLACK_APP_TOKEN").ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_tokens_are_used_verbatim() {
        let cfg = SlackConfig {
            bot_token: Some("xoxb-test-bot".to_string()),
            app_token: Some("xapp-test-app".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.resolved_bot_token().as_deref(), Some("xoxb-test-bot"));
        assert_eq!(cfg.resolved_app_token().as_deref(), Some("xapp-test-app"));
    }

    #[test]
    fn allowed_channels_empty_by_default() {
        let cfg = SlackConfig::default();
        assert!(cfg.allowed_channels.is_empty());
    }

    #[test]
    fn allowed_users_empty_by_default() {
        let cfg = SlackConfig::default();
        assert!(cfg.allowed_users.is_empty());
    }

    #[test]
    fn slack_config_roundtrips_toml() {
        let toml_str = r#"
            bot_token = "xoxb-123"
            app_token = "xapp-456"
            allowed_channels = ["C001", "C002"]
            allowed_users = ["U001"]
        "#;
        let cfg: SlackConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(cfg.bot_token.as_deref(), Some("xoxb-123"));
        assert_eq!(cfg.app_token.as_deref(), Some("xapp-456"));
        assert_eq!(cfg.allowed_channels, ["C001", "C002"]);
        assert_eq!(cfg.allowed_users, ["U001"]);
    }

    #[test]
    fn assistant_config_with_slack_section_roundtrips() {
        let toml_str = r#"
            [slack]
            bot_token = "xoxb-abc"
            allowed_channels = ["CGENERAL"]
        "#;
        let cfg: assistant_core::AssistantConfig = toml::from_str(toml_str).expect("parse");
        let slack = cfg.slack.expect("slack section present");
        assert_eq!(slack.bot_token.as_deref(), Some("xoxb-abc"));
        assert_eq!(slack.allowed_channels, ["CGENERAL"]);
    }

    #[test]
    fn assistant_config_without_slack_section_is_none() {
        let cfg: assistant_core::AssistantConfig = toml::from_str("").expect("parse");
        assert!(cfg.slack.is_none());
    }
}
