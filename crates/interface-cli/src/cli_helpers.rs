//! Small CLI helpers used by `main`: the interactive confirmation prompt, and worker-interface name normalisation/filtering.

use std::collections::HashSet;
use std::io::{self, Write as IoWrite};

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
}
