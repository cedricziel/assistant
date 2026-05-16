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
