//! `[permissions]` configuration section.

use serde::{Deserialize, Serialize};

use crate::permission::Rule;

/// Tool-permission configuration (`[permissions]` in `config.toml`).
///
/// Rules use the grammar `tool-name` or `tool-name(pattern)`; see
/// [`crate::permission`]. Config rules are *added* to the built-in rules;
/// they cannot remove a built-in deny.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionsConfig {
    /// Rules that always deny. Evaluated first; never reach the decision model.
    #[serde(default)]
    pub deny: Vec<Rule>,
    /// Rules that force the human-confirmation path.
    #[serde(default)]
    pub ask: Vec<Rule>,
    /// Rules that allow without further checks (deny and ask still win).
    #[serde(default)]
    pub allow: Vec<Rule>,
    /// Extra directories (besides the persona workspace) file tools may use.
    #[serde(default)]
    pub workspace_roots: Vec<String>,
    /// Resource limits for `bash` and `process`.
    #[serde(default)]
    pub limits: LimitsConfig,
}

/// Resource limits enforced by the `bash` and `process` tools
/// (`[permissions.limits]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LimitsConfig {
    /// Upper bound for `bash` `timeout_secs`; larger requests are clamped.
    pub bash_max_timeout_secs: u64,
    /// Characters of stdout/stderr `bash` keeps (the tail); the rest is dropped
    /// while the command runs.
    pub bash_max_output_chars: usize,
    /// A `process` session idle this long is killed and forgotten.
    pub process_ttl_secs: u64,
    /// Maximum number of running `process` sessions.
    pub process_max_sessions: usize,
    /// Stored `process` output lines are truncated to this many characters.
    pub process_max_line_chars: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            bash_max_timeout_secs: 600,
            bash_max_output_chars: 200_000,
            process_ttl_secs: 3_600,
            process_max_sessions: 16,
            process_max_line_chars: 4_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_from_toml() {
        let cfg: PermissionsConfig = toml::from_str(
            r#"
deny = ["bash(rm -rf *)"]
ask = ["bash(git push*)"]
allow = ["bash(git status*)"]
workspace_roots = ["~/scratch"]
"#,
        )
        .unwrap();
        assert_eq!(cfg.deny[0].as_str(), "bash(rm -rf *)");
        assert_eq!(cfg.workspace_roots, vec!["~/scratch"]);
    }

    #[test]
    fn invalid_rule_fails_naming_the_rule() {
        let err = toml::from_str::<PermissionsConfig>("allow = [\"bash(oops\"]")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("bash(oops"),
            "error should name the rule: {err}"
        );
    }

    #[test]
    fn limits_default_and_override() {
        let cfg: PermissionsConfig =
            toml::from_str("[limits]\nbash_max_timeout_secs = 30").unwrap();
        assert_eq!(cfg.limits.bash_max_timeout_secs, 30);
        assert_eq!(cfg.limits.process_max_sessions, 16);
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<PermissionsConfig>("alow = []").is_err());
    }

    #[test]
    fn assistant_config_defaults_to_empty_permissions() {
        let cfg: crate::types::agent::AssistantConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.permissions, PermissionsConfig::default());
    }
}
