//! [`Secret`]: a string newtype for credentials that never leaks through
//! `Debug` or `Display`.

use serde::{Deserialize, Serialize};

const REDACTED: &str = "[REDACTED]";

/// A credential (API key, bot token, password, shared secret).
///
/// `Debug` and `Display` print a fixed placeholder so a stray `{:?}` of a
/// config struct cannot leak the value into logs. Serde is transparent, so
/// config files read and write the plain string. Use [`Secret::expose`] at the
/// point where the value is actually sent to a remote service.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw credential. Call only where it is transmitted.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// `true` when the credential is empty or whitespace only.
    pub fn is_blank(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED)
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED)
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_redact() {
        let s = Secret::new("hunter2");
        assert_eq!(format!("{s:?}"), "[REDACTED]", "Debug must redact");
        assert_eq!(format!("{s}"), "[REDACTED]", "Display must redact");
    }

    #[test]
    fn expose_returns_raw_value() {
        assert_eq!(Secret::new("hunter2").expose(), "hunter2");
    }

    #[test]
    fn serde_is_transparent() {
        let s: Secret = serde_json::from_str("\"hunter2\"").unwrap();
        assert_eq!(s.expose(), "hunter2", "deserializes from a bare string");
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            "\"hunter2\"",
            "serializes to a bare string"
        );
    }

    #[test]
    fn toml_round_trip_inside_option() {
        #[derive(Serialize, Deserialize)]
        struct C {
            key: Option<Secret>,
        }
        let c: C = toml::from_str("key = \"abc\"").unwrap();
        assert_eq!(c.key.as_ref().map(Secret::expose), Some("abc"));
        assert_eq!(toml::to_string(&c).unwrap().trim(), "key = \"abc\"");
    }

    #[test]
    fn blank_detection() {
        assert!(Secret::new("  ").is_blank());
        assert!(!Secret::new("x").is_blank());
    }
}
