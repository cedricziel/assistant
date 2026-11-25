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
    /// Decision model consulted for calls the static rules leave undecided.
    #[serde(default)]
    pub decision_model: DecisionModelConfig,
}

/// The decision model (`[permissions.decision_model]`).
///
/// Disabled by default. When enabled, calls no static rule decides are sent to
/// a Jev model; a call is approved only if every answer scores at least
/// `threshold`. Anything else (low score, error, timeout) falls back to the
/// human / deny path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DecisionModelConfig {
    pub enabled: bool,
    /// Pinned model id, e.g. `typesafe/jev-1.13`. `latest` aliases are rejected.
    #[serde(deserialize_with = "deserialize_pinned_model")]
    pub model: String,
    /// Minimum score (0 < t <= 1) every answer must reach to approve.
    #[serde(deserialize_with = "deserialize_threshold")]
    pub threshold: f32,
    /// Request timeout; a timeout counts as "not approved".
    pub timeout_secs: u64,
    /// Decisions endpoint.
    pub base_url: String,
    /// Environment variable holding the API key.
    pub api_key_env: String,
}

impl Default for DecisionModelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: "typesafe/jev-1.13".to_string(),
            threshold: 0.9,
            timeout_secs: 8,
            base_url: "https://openrouter.ai/api/alpha/decisions".to_string(),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
        }
    }
}

fn deserialize_pinned_model<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let model = String::deserialize(d)?;
    let version = model.rsplit('-').next().unwrap_or("");
    if model.to_ascii_lowercase().contains("latest")
        || !version.starts_with(|c: char| c.is_ascii_digit())
    {
        return Err(serde::de::Error::custom(format!(
            "decision_model.model '{model}' is not pinned to a version (expected e.g. typesafe/jev-1.13)"
        )));
    }
    Ok(model)
}

fn deserialize_threshold<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
    let t = f32::deserialize(d)?;
    if t > 0.0 && t <= 1.0 {
        Ok(t)
    } else {
        Err(serde::de::Error::custom(format!(
            "decision_model.threshold {t} must be in (0, 1]"
        )))
    }
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
    fn decision_model_is_disabled_and_pinned_by_default() {
        let cfg = PermissionsConfig::default().decision_model;
        assert!(!cfg.enabled);
        assert_eq!(cfg.model, "typesafe/jev-1.13");
        assert_eq!(cfg.threshold, 0.9);
    }

    #[test]
    fn decision_model_parses_overrides() {
        let cfg: PermissionsConfig = toml::from_str(
            "[decision_model]\nenabled = true\nmodel = \"typesafe/jev-2.0\"\nthreshold = 0.8\ntimeout_secs = 3",
        )
        .unwrap();
        assert!(cfg.decision_model.enabled);
        assert_eq!(cfg.decision_model.model, "typesafe/jev-2.0");
        assert_eq!(cfg.decision_model.timeout_secs, 3);
    }

    #[test]
    fn unpinned_model_ids_are_rejected() {
        for bad in [
            "typesafe/jev-latest",
            "typesafe/JEV-LATEST",
            "typesafe/jev",
            "jev-",
        ] {
            let toml = format!("[decision_model]\nmodel = \"{bad}\"");
            let err = toml::from_str::<PermissionsConfig>(&toml)
                .unwrap_err()
                .to_string();
            assert!(err.contains("not pinned"), "{bad}: {err}");
        }
    }

    #[test]
    fn out_of_range_thresholds_are_rejected() {
        for bad in ["0", "1.5", "-0.1"] {
            let toml = format!("[decision_model]\nthreshold = {bad}");
            assert!(toml::from_str::<PermissionsConfig>(&toml).is_err(), "{bad}");
        }
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<PermissionsConfig>("alow = []").is_err());
    }

    #[test]
    fn assistant_config_defaults_to_empty_permissions() {
        let cfg: crate::types::agent::OrgConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.permissions, PermissionsConfig::default());
    }
}
