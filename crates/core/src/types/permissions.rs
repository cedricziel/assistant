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
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<PermissionsConfig>("alow = []").is_err());
    }

    #[test]
    fn assistant_config_defaults_to_empty_permissions() {
        let cfg: crate::types::agent::AssistantConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.permissions, PermissionsConfig::default());
    }
}
