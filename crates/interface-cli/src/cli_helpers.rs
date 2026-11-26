//! Small CLI helpers used by `main`: the interactive confirmation prompt, and worker-interface name normalisation/filtering.

use std::collections::HashSet;
use std::io::{self, Write as IoWrite};
use std::sync::Arc;

use async_trait::async_trait;

use assistant_runtime::orchestrator::{
    ConfirmationAnswer, ConfirmationCallback, ConfirmationRequest,
};

/// Source of one line of operator input; blocking.
type ReadLine = dyn Fn() -> io::Result<String> + Send + Sync;

/// Interactive confirmation callback for tool execution — prints a prompt
/// and reads a line from stdin without blocking the async runtime.
///
/// Offers only the once answers: a terminal has no store for saved rules.
pub struct CliConfirmation {
    read_line: Arc<ReadLine>,
}

impl CliConfirmation {
    /// Read answers from the process's stdin.
    pub fn stdin() -> Self {
        Self::with_reader(|| {
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            Ok(buf)
        })
    }

    /// Read answers from `read_line`, which may block.
    pub fn with_reader(read_line: impl Fn() -> io::Result<String> + Send + Sync + 'static) -> Self {
        Self {
            read_line: Arc::new(read_line),
        }
    }
}

#[async_trait]
impl ConfirmationCallback for CliConfirmation {
    async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
        let params_str = serde_json::to_string_pretty(req.params).unwrap_or_default();
        print!(
            "\nTool '{}' requires confirmation.\nParams: {}\nProceed? [y/N] ",
            req.tool_name, params_str
        );
        io::stdout().flush().ok();

        let read_line = self.read_line.clone();
        match tokio::task::spawn_blocking(move || read_line()).await {
            Ok(Ok(line)) if matches!(line.trim().to_lowercase().as_str(), "y" | "yes") => {
                ConfirmationAnswer::AllowOnce
            }
            Ok(_) => ConfirmationAnswer::RejectOnce,
            Err(_) => ConfirmationAnswer::Cancelled,
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

    use assistant_core::types::conversation::{ExecutionContext, Interface};
    use assistant_runtime::orchestrator::ApprovalOptions;
    use uuid::Uuid;

    async fn ask(cb: &CliConfirmation) -> ConfirmationAnswer {
        let ctx = ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "cli-test".into(),
            turn: 0,
            interface: Interface::Cli,
            interactive: true,
            run_id: None,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        };
        cb.confirm(ConfirmationRequest {
            tool_name: "bash",
            params: &serde_json::json!({ "command": "ls" }),
            ctx: &ctx,
            tool_call_id: None,
            reason: "undecided",
            options: ApprovalOptions::once_only(),
            suggested_rule: None,
            event_sink: None,
        })
        .await
    }

    fn typed(line: &'static str) -> CliConfirmation {
        CliConfirmation::with_reader(move || Ok(line.to_string()))
    }

    #[tokio::test]
    async fn yes_answers_allow_once() {
        for line in ["y\n", "YES\n", " yes \n"] {
            assert_eq!(
                ask(&typed(line)).await,
                ConfirmationAnswer::AllowOnce,
                "{line:?}"
            );
        }
    }

    #[tokio::test]
    async fn anything_else_rejects_once() {
        for line in ["n\n", "\n", "", "always\n"] {
            assert_eq!(
                ask(&typed(line)).await,
                ConfirmationAnswer::RejectOnce,
                "{line:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_read_error_rejects_once() {
        let cb = CliConfirmation::with_reader(|| Err(io::Error::other("closed")));
        assert_eq!(ask(&cb).await, ConfirmationAnswer::RejectOnce);
    }

    #[tokio::test]
    async fn a_panicking_reader_cancels() {
        let cb = CliConfirmation::with_reader(|| panic!("reader crashed"));
        assert_eq!(ask(&cb).await, ConfirmationAnswer::Cancelled);
    }

    /// On a single-threaded runtime a blocking read on the runtime thread
    /// would starve the task that unblocks it, so this only finishes when the
    /// read runs off the async runtime.
    #[tokio::test(flavor = "current_thread")]
    async fn reading_does_not_block_the_async_runtime() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let rx = std::sync::Mutex::new(rx);
        let cb = CliConfirmation::with_reader(move || {
            rx.lock()
                .map_err(|_| io::Error::other("poisoned"))?
                .recv()
                .map_err(io::Error::other)
        });
        let typer = tokio::spawn(async move {
            tokio::task::yield_now().await;
            tx.send("y\n".to_string()).ok();
        });
        let answer = tokio::time::timeout(std::time::Duration::from_secs(5), ask(&cb)).await;
        typer.await.unwrap();
        assert_eq!(answer.unwrap(), ConfirmationAnswer::AllowOnce);
    }

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
