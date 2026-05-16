//! Matrix interface configuration.
//!
//! [`MatrixConfig`] is defined in `assistant-core` so it can be embedded
//! in [`AssistantConfig`][assistant_core::AssistantConfig].  This module
//! re-exports it and adds runtime helpers that depend on env vars.

pub use assistant_core::types::channels::MatrixConfig;

/// Extension methods for [`MatrixConfig`] that resolve values from env vars.
#[allow(dead_code)]
pub trait MatrixConfigExt {
    /// Return the homeserver URL, falling back to `MATRIX_HOMESERVER_URL`.
    fn resolved_homeserver_url(&self) -> Option<String>;

    /// Return the bot username, falling back to `MATRIX_USERNAME`.
    fn resolved_username(&self) -> Option<String>;

    /// Return the access token, falling back to `MATRIX_ACCESS_TOKEN`.
    fn resolved_access_token(&self) -> Option<String>;

    /// Return the password, falling back to `MATRIX_PASSWORD`.
    fn resolved_password(&self) -> Option<String>;

    /// Return the state store path (no env-var fallback; uses runtime default).
    fn resolved_state_store_path(&self) -> Option<String>;
}

impl MatrixConfigExt for MatrixConfig {
    fn resolved_homeserver_url(&self) -> Option<String> {
        self.homeserver_url
            .clone()
            .or_else(|| std::env::var("MATRIX_HOMESERVER_URL").ok())
    }

    fn resolved_username(&self) -> Option<String> {
        self.username
            .clone()
            .or_else(|| std::env::var("MATRIX_USERNAME").ok())
    }

    fn resolved_access_token(&self) -> Option<String> {
        self.access_token
            .clone()
            .or_else(|| std::env::var("MATRIX_ACCESS_TOKEN").ok())
    }

    fn resolved_password(&self) -> Option<String> {
        self.password
            .clone()
            .or_else(|| std::env::var("MATRIX_PASSWORD").ok())
    }

    #[allow(dead_code)]
    fn resolved_state_store_path(&self) -> Option<String> {
        self.state_store_path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_values_are_used_verbatim() {
        let cfg = MatrixConfig {
            homeserver_url: Some("https://matrix.example.com".to_string()),
            username: Some("@bot:example.com".to_string()),
            access_token: Some("tok-abc".to_string()),
            password: Some("s3cr3t".to_string()),
            ..Default::default()
        };
        assert_eq!(
            cfg.resolved_homeserver_url().as_deref(),
            Some("https://matrix.example.com"),
            "homeserver_url should be returned verbatim"
        );
        assert_eq!(
            cfg.resolved_username().as_deref(),
            Some("@bot:example.com"),
            "username should be returned verbatim"
        );
        assert_eq!(
            cfg.resolved_access_token().as_deref(),
            Some("tok-abc"),
            "access_token should be returned verbatim"
        );
        assert_eq!(
            cfg.resolved_password().as_deref(),
            Some("s3cr3t"),
            "password should be returned verbatim"
        );
    }

    #[test]
    fn allowed_rooms_empty_by_default() {
        let cfg = MatrixConfig::default();
        assert!(cfg.allowed_rooms.is_empty(), "allowed_rooms must be empty");
    }

    #[test]
    fn allowed_users_empty_by_default() {
        let cfg = MatrixConfig::default();
        assert!(cfg.allowed_users.is_empty(), "allowed_users must be empty");
    }

    #[test]
    fn matrix_config_roundtrips_toml() {
        let toml_str = r#"
            homeserver_url = "https://matrix.example.com"
            username = "@bot:example.com"
            access_token = "tok-xyz"
            device_id = "MYDEVICE"
            state_store_path = "/tmp/matrix-state"
            allowed_rooms = ["!abc:example.com", "!def:example.com"]
            allowed_users = ["@alice:example.com"]
        "#;
        let cfg: MatrixConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(
            cfg.homeserver_url.as_deref(),
            Some("https://matrix.example.com"),
            "homeserver_url"
        );
        assert_eq!(
            cfg.username.as_deref(),
            Some("@bot:example.com"),
            "username"
        );
        assert_eq!(cfg.access_token.as_deref(), Some("tok-xyz"), "access_token");
        assert_eq!(cfg.device_id.as_deref(), Some("MYDEVICE"), "device_id");
        assert_eq!(
            cfg.state_store_path.as_deref(),
            Some("/tmp/matrix-state"),
            "state_store_path"
        );
        assert_eq!(
            cfg.allowed_rooms,
            ["!abc:example.com", "!def:example.com"],
            "allowed_rooms"
        );
        assert_eq!(cfg.allowed_users, ["@alice:example.com"], "allowed_users");
    }

    #[test]
    fn assistant_config_with_matrix_section_roundtrips() {
        let toml_str = r#"
            [matrix]
            homeserver_url = "https://matrix.example.com"
            access_token = "tok-abc"
        "#;
        let cfg: assistant_core::AssistantConfig = toml::from_str(toml_str).expect("parse");
        let mx = cfg.matrix.expect("matrix section present");
        assert_eq!(
            mx.homeserver_url.as_deref(),
            Some("https://matrix.example.com"),
            "homeserver_url from AssistantConfig"
        );
        assert_eq!(
            mx.access_token.as_deref(),
            Some("tok-abc"),
            "access_token from AssistantConfig"
        );
    }

    #[test]
    fn assistant_config_without_matrix_section_is_none() {
        let cfg: assistant_core::AssistantConfig = toml::from_str("").expect("parse");
        assert!(cfg.matrix.is_none(), "matrix should be None when absent");
    }
}
