//! Pure selection logic for `main`: which [`BinaryRole`], confirmation mode,
//! messenger interfaces and filtered workers a given command/config picks.

use std::collections::HashSet;

use assistant_core::types::agent::AssistantConfig;
use assistant_runtime::BinaryRole;

use crate::args::Command;
use crate::cli_helpers::interface_selected;

/// A chat messenger the binary can run, either as the only interface
/// (`assistant slack`) or in the background next to the REPL/orchestrator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Messenger {
    Slack,
    Mattermost,
    Matrix,
    Nextcloud,
    Signal,
}

/// How a messenger is hosted by this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// `assistant <messenger>`: the messenger is the process; no REPL.
    Dedicated,
    /// REPL / orchestrator mode: the messenger runs as a background task.
    Background,
}

/// An interface-filtered turn worker to spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilteredWorker {
    pub id: &'static str,
    pub interface: &'static str,
}

impl Messenger {
    /// Background start order.
    pub const ALL: [Messenger; 5] = [
        Messenger::Slack,
        Messenger::Mattermost,
        Messenger::Matrix,
        Messenger::Nextcloud,
        Messenger::Signal,
    ];

    pub fn from_command(command: Option<&Command>) -> Option<Self> {
        match command? {
            Command::Slack => Some(Self::Slack),
            Command::Mattermost => Some(Self::Mattermost),
            Command::Matrix => Some(Self::Matrix),
            Command::Nextcloud => Some(Self::Nextcloud),
            Command::Signal => Some(Self::Signal),
            _ => None,
        }
    }

    /// Lower-case name used in `--interfaces`.
    pub fn key(self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Mattermost => "mattermost",
            Self::Matrix => "matrix",
            Self::Nextcloud => "nextcloud",
            Self::Signal => "signal",
        }
    }

    /// `Interface` name the worker filters on.
    pub fn interface_name(self) -> &'static str {
        match self {
            Self::Slack => "Slack",
            Self::Mattermost => "Mattermost",
            Self::Matrix => "Matrix",
            Self::Nextcloud => "Nextcloud",
            Self::Signal => "Signal",
        }
    }

    fn worker_id(self) -> &'static str {
        match self {
            Self::Slack => "slack-worker",
            Self::Mattermost => "mattermost-worker",
            Self::Matrix => "matrix-worker",
            Self::Nextcloud => "nextcloud-worker",
            Self::Signal => "signal-worker",
        }
    }

    pub fn is_configured(self, config: &AssistantConfig) -> bool {
        match self {
            Self::Slack => config.slack.is_some(),
            Self::Mattermost => config.mattermost.is_some(),
            Self::Matrix => config.matrix.is_some(),
            Self::Nextcloud => config.nextcloud.is_some(),
            Self::Signal => config.signal.is_some(),
        }
    }

    pub fn workers(self, mode: RunMode) -> Vec<FilteredWorker> {
        let own = FilteredWorker {
            id: self.worker_id(),
            interface: self.interface_name(),
        };
        match mode {
            // The unfiltered main worker is not started in dedicated mode, so
            // scheduled tasks need their own consumer.
            RunMode::Dedicated => vec![
                own,
                FilteredWorker {
                    id: "scheduler-worker",
                    interface: "Scheduler",
                },
            ],
            RunMode::Background => vec![own],
        }
    }
}

pub fn select_binary_role(command: Option<&Command>, interface_filtered: bool) -> BinaryRole {
    #[cfg(feature = "mcp")]
    if matches!(command, Some(Command::Mcp)) {
        return BinaryRole::Mcp;
    }
    match command {
        Some(Command::Worker { .. }) => BinaryRole::WorkerOnly,
        _ if Messenger::from_command(command).is_some() => BinaryRole::InterfaceOnly,
        Some(Command::Orchestrator { .. }) if interface_filtered => {
            BinaryRole::OrchestratorFiltered
        }
        Some(Command::Orchestrator { .. }) => BinaryRole::OrchestratorUnfiltered,
        _ => BinaryRole::Repl,
    }
}

/// Whether tool confirmations are auto-denied (no terminal interaction).
pub fn uses_auto_deny_confirmation(command: Option<&Command>, no_repl: bool) -> bool {
    #[cfg(feature = "mcp")]
    if matches!(command, Some(Command::Mcp)) {
        return true;
    }
    // Matrix is deliberately absent: it has never been auto-denied.
    let dedicated = matches!(
        Messenger::from_command(command),
        Some(Messenger::Slack | Messenger::Mattermost | Messenger::Nextcloud | Messenger::Signal)
    );
    dedicated || no_repl
}

/// Messengers to start in background mode, in start order.
pub fn background_messengers(
    config: &AssistantConfig,
    selection: &HashSet<String>,
) -> Vec<Messenger> {
    Messenger::ALL
        .into_iter()
        .filter(|m| m.is_configured(config) && interface_selected(selection, m.key()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Cli;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("assistant").chain(args.iter().copied())).unwrap()
    }

    fn role(args: &[&str], filtered: bool) -> BinaryRole {
        select_binary_role(parse(args).command.as_ref(), filtered)
    }

    #[test]
    fn no_subcommand_is_repl() {
        assert_eq!(role(&[], false), BinaryRole::Repl);
    }

    #[test]
    fn messenger_subcommands_are_interface_only() {
        for name in ["slack", "mattermost", "nextcloud", "matrix", "signal"] {
            assert_eq!(role(&[name], false), BinaryRole::InterfaceOnly, "{name}");
        }
    }

    #[test]
    fn worker_subcommand_is_worker_only() {
        assert_eq!(role(&["worker"], false), BinaryRole::WorkerOnly);
    }

    #[test]
    fn orchestrator_role_depends_on_interface_filter() {
        assert_eq!(
            role(&["orchestrator", "run"], false),
            BinaryRole::OrchestratorUnfiltered
        );
        assert_eq!(
            role(&["orchestrator", "run"], true),
            BinaryRole::OrchestratorFiltered
        );
    }

    #[test]
    fn other_subcommands_fall_back_to_repl() {
        assert_eq!(role(&["status"], false), BinaryRole::Repl);
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_subcommand_is_mcp_role() {
        assert_eq!(role(&["mcp"], false), BinaryRole::Mcp);
    }

    fn auto_deny(args: &[&str], no_repl: bool) -> bool {
        uses_auto_deny_confirmation(parse(args).command.as_ref(), no_repl)
    }

    #[test]
    fn repl_confirms_interactively() {
        assert!(!auto_deny(&[], false));
    }

    #[test]
    fn dedicated_messengers_auto_deny_except_matrix() {
        for name in ["slack", "mattermost", "nextcloud", "signal"] {
            assert!(auto_deny(&[name], false), "{name}");
        }
        // Pins today's behaviour: Matrix is not in the auto-deny set.
        assert!(!auto_deny(&["matrix"], false));
    }

    #[test]
    fn no_repl_auto_denies() {
        assert!(auto_deny(&["orchestrator", "run", "--no-repl"], true));
        assert!(!auto_deny(&["orchestrator", "run"], false));
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_auto_denies() {
        assert!(auto_deny(&["mcp"], false));
    }

    #[test]
    fn messenger_from_command() {
        let cases = [
            ("slack", Messenger::Slack),
            ("mattermost", Messenger::Mattermost),
            ("matrix", Messenger::Matrix),
            ("nextcloud", Messenger::Nextcloud),
            ("signal", Messenger::Signal),
        ];
        for (arg, expected) in cases {
            assert_eq!(
                Messenger::from_command(parse(&[arg]).command.as_ref()),
                Some(expected)
            );
        }
        assert_eq!(Messenger::from_command(parse(&[]).command.as_ref()), None);
        assert_eq!(
            Messenger::from_command(parse(&["worker"]).command.as_ref()),
            None
        );
    }

    #[test]
    fn messenger_names() {
        let names: Vec<_> = Messenger::ALL
            .iter()
            .map(|m| (m.key(), m.interface_name()))
            .collect();
        assert_eq!(
            names,
            vec![
                ("slack", "Slack"),
                ("mattermost", "Mattermost"),
                ("matrix", "Matrix"),
                ("nextcloud", "Nextcloud"),
                ("signal", "Signal"),
            ]
        );
    }

    #[test]
    fn dedicated_workers_add_scheduler_worker() {
        assert_eq!(
            Messenger::Slack.workers(RunMode::Dedicated),
            vec![
                FilteredWorker {
                    id: "slack-worker",
                    interface: "Slack"
                },
                FilteredWorker {
                    id: "scheduler-worker",
                    interface: "Scheduler"
                },
            ]
        );
    }

    #[test]
    fn background_workers_are_messenger_only() {
        for m in Messenger::ALL {
            let workers = m.workers(RunMode::Background);
            assert_eq!(
                workers,
                vec![FilteredWorker {
                    id: match m {
                        Messenger::Slack => "slack-worker",
                        Messenger::Mattermost => "mattermost-worker",
                        Messenger::Matrix => "matrix-worker",
                        Messenger::Nextcloud => "nextcloud-worker",
                        Messenger::Signal => "signal-worker",
                    },
                    interface: m.interface_name()
                }]
            );
        }
    }

    fn config_with(messengers: &[Messenger]) -> AssistantConfig {
        let mut c = AssistantConfig::default();
        for m in messengers {
            match m {
                Messenger::Slack => c.slack = Some(Default::default()),
                Messenger::Mattermost => c.mattermost = Some(Default::default()),
                Messenger::Matrix => c.matrix = Some(Default::default()),
                Messenger::Nextcloud => c.nextcloud = Some(Default::default()),
                Messenger::Signal => c.signal = Some(Default::default()),
            }
        }
        c
    }

    #[test]
    fn background_selects_configured_in_start_order() {
        let cfg = config_with(&[Messenger::Signal, Messenger::Slack, Messenger::Matrix]);
        assert_eq!(
            background_messengers(&cfg, &HashSet::new()),
            vec![Messenger::Slack, Messenger::Matrix, Messenger::Signal]
        );
    }

    #[test]
    fn background_honours_selection() {
        let cfg = config_with(&Messenger::ALL);
        let sel: HashSet<String> = ["matrix", "slack"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            background_messengers(&cfg, &sel),
            vec![Messenger::Slack, Messenger::Matrix]
        );
    }

    #[test]
    fn background_skips_unconfigured_even_if_selected() {
        let cfg = config_with(&[Messenger::Slack]);
        let sel: HashSet<String> = ["slack", "signal"].iter().map(|s| s.to_string()).collect();
        assert_eq!(background_messengers(&cfg, &sel), vec![Messenger::Slack]);
    }

    #[test]
    fn nothing_configured_selects_nothing() {
        assert!(background_messengers(&AssistantConfig::default(), &HashSet::new()).is_empty());
    }
}
