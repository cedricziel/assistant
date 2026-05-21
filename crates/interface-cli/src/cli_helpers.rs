//! Small CLI helpers used by `main`: config loading, interactive
//! confirmation prompt, and worker-interface name normalisation/filtering.

use std::collections::HashSet;
use std::io::{self, Write as IoWrite};
use std::path::Path;

use assistant_core::types::agent::AssistantConfig;
use assistant_runtime::orchestrator::ConfirmationCallback;

/// Interactive confirmation callback for tool execution — prints a prompt
/// and reads a line from stdin.
pub struct CliConfirmation;

impl ConfirmationCallback for CliConfirmation {
    fn confirm(&self, skill_name: &str, params: &serde_json::Value) -> bool {
        let params_str = serde_json::to_string_pretty(params).unwrap_or_default();
        print!(
            "\nTool '{}' requires confirmation.\nParams: {}\nProceed? [y/N] ",
            skill_name, params_str
        );
        io::stdout().flush().ok();

        let mut buf = String::new();
        if io::stdin().read_line(&mut buf).is_err() {
            return false;
        }
        matches!(buf.trim().to_lowercase().as_str(), "y" | "yes")
    }
}

// ── Config loading ────────────────────────────────────────────────────────────

pub enum ConfigLoadMessage {
    Info(String),
    Warn(String),
}

pub fn load_config_messages(config_path: &Path) -> (AssistantConfig, Vec<ConfigLoadMessage>) {
    if !config_path.exists() {
        return (AssistantConfig::default(), Vec::new());
    }

    let mut messages = Vec::new();
    let raw = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            messages.push(ConfigLoadMessage::Warn(format!(
                "Failed to read config at {}: {e}",
                config_path.display()
            )));
            return (AssistantConfig::default(), messages);
        }
    };

    match toml::from_str::<AssistantConfig>(&raw) {
        Ok(cfg) => {
            messages.push(ConfigLoadMessage::Info(format!(
                "Loaded config from {}",
                config_path.display()
            )));
            (cfg, messages)
        }
        Err(e) => {
            messages.push(ConfigLoadMessage::Warn(format!(
                "Failed to parse config at {}: {e}",
                config_path.display()
            )));
            (AssistantConfig::default(), messages)
        }
    }
}

// ── Worker-interface name normalisation ───────────────────────────────────────

pub fn normalize_worker_interface(interface: &str) -> Option<String> {
    match interface.trim().to_lowercase().as_str() {
        "" | "any" | "all" => None,
        "slack" => Some("Slack".to_string()),
        "mattermost" => Some("Mattermost".to_string()),
        "nextcloud" => Some("Nextcloud".to_string()),
        "web" | "webui" => Some("Web".to_string()),
        "signal" => Some("Signal".to_string()),
        other => Some(other.to_string()),
    }
}

pub fn parse_interface_selection(input: Option<&str>) -> HashSet<String> {
    input
        .map(|raw| {
            raw.split(',')
                .map(|v| v.trim().to_lowercase())
                .filter(|v| !v.is_empty())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default()
}

pub fn interface_selected(selected: &HashSet<String>, interface: &str) -> bool {
    selected.is_empty() || selected.contains(interface)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_worker_interface_canonicalises_known_names() {
        assert_eq!(normalize_worker_interface("slack"), Some("Slack".into()));
        assert_eq!(normalize_worker_interface("SLACK"), Some("Slack".into()));
        assert_eq!(
            normalize_worker_interface("mattermost"),
            Some("Mattermost".into())
        );
        assert_eq!(
            normalize_worker_interface("nextcloud"),
            Some("Nextcloud".into())
        );
        assert_eq!(normalize_worker_interface("web"), Some("Web".into()));
        assert_eq!(normalize_worker_interface("webui"), Some("Web".into()));
        assert_eq!(normalize_worker_interface("signal"), Some("Signal".into()));
    }

    #[test]
    fn normalize_worker_interface_returns_none_for_any_all_or_empty() {
        assert_eq!(normalize_worker_interface(""), None);
        assert_eq!(normalize_worker_interface("any"), None);
        assert_eq!(normalize_worker_interface("all"), None);
        assert_eq!(normalize_worker_interface("  ALL  "), None);
    }

    #[test]
    fn normalize_worker_interface_returns_lowercased_unknown_as_is() {
        // Unknown interfaces fall through to the lowercase value.
        assert_eq!(normalize_worker_interface("custom"), Some("custom".into()));
    }

    #[test]
    fn parse_interface_selection_splits_and_trims() {
        let s = parse_interface_selection(Some(" slack , mattermost , web "));
        assert!(s.contains("slack"));
        assert!(s.contains("mattermost"));
        assert!(s.contains("web"));
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn parse_interface_selection_drops_empty_chunks() {
        let s = parse_interface_selection(Some(" , slack , "));
        assert_eq!(s.len(), 1);
        assert!(s.contains("slack"));
    }

    #[test]
    fn parse_interface_selection_none_yields_empty() {
        let s = parse_interface_selection(None);
        assert!(s.is_empty());
    }

    #[test]
    fn interface_selected_empty_means_all_selected() {
        let empty = HashSet::new();
        assert!(interface_selected(&empty, "anything"));
    }

    #[test]
    fn interface_selected_filters_when_non_empty() {
        let mut s = HashSet::new();
        s.insert("slack".to_string());
        assert!(interface_selected(&s, "slack"));
        assert!(!interface_selected(&s, "web"));
    }

    #[test]
    fn load_config_messages_returns_default_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.toml");
        let (cfg, msgs) = load_config_messages(&path);
        assert_eq!(cfg.agent.id, AssistantConfig::default().agent.id);
        assert!(msgs.is_empty());
    }

    #[test]
    fn load_config_messages_warns_on_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, b"this = is :: invalid ::: toml").unwrap();

        let (_cfg, msgs) = load_config_messages(&path);
        assert!(msgs.iter().any(|m| matches!(m, ConfigLoadMessage::Warn(_))));
    }

    #[test]
    fn load_config_messages_emits_info_on_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[agent]\nid = \"my-agent\"\n").unwrap();

        let (cfg, msgs) = load_config_messages(&path);
        assert_eq!(cfg.agent.id, "my-agent");
        assert!(msgs.iter().any(|m| matches!(m, ConfigLoadMessage::Info(_))));
    }
}
