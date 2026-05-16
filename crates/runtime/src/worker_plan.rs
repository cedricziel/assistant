//! Worker-spawning plan for each binary role.
//!
//! The unified `assistant` binary boots into several different modes (REPL,
//! orchestrator daemon, single-interface daemon, MCP server, dedicated worker,
//! web-UI). Each mode decides which **infrastructure** workers to spawn at
//! startup:
//!
//! - `main-worker` — unfiltered turn consumer (REPL / orchestrator-unfiltered).
//! - `scheduler-worker` — claims `Scheduler`-interface turns.
//! - `web-worker` — claims `Web`-interface turns.
//! - `TitleGeneratorWorker` — consumes `turn.result` and auto-titles
//!   conversations.
//! - `memory_indexer` — periodic memory embedding pipeline.
//!
//! Interface-specific workers (`slack-worker`, `matrix-worker`, etc.) are
//! co-located with their adapter init code and aren't covered here — they
//! exist iff the adapter runs in this process.
//!
//! This helper exists so the binary-level wiring can be verified by unit
//! tests instead of subprocess smoke tests. Call sites in
//! `crates/interface-cli/src/main.rs` and `crates/web-ui/src/main.rs`
//! consume the plan and spawn exactly what it returns.

/// One infrastructure worker spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreWorkerSpec {
    /// Worker identifier passed to `Orchestrator::run_worker_filtered`.
    pub worker_id: &'static str,
    /// Optional interface filter for `ClaimFilter::with_interface`. `None`
    /// means an unfiltered worker that claims every interface.
    pub interface_filter: Option<&'static str>,
}

/// The full set of infrastructure workers + companion services a binary
/// should spawn at startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreWorkerPlan {
    pub workers: Vec<CoreWorkerSpec>,
    pub spawn_title_generator: bool,
    pub spawn_memory_indexer: bool,
}

impl CoreWorkerPlan {
    /// Returns `true` if the plan asks for any worker / companion service.
    pub fn is_empty(&self) -> bool {
        self.workers.is_empty() && !self.spawn_title_generator && !self.spawn_memory_indexer
    }

    /// Convenience: does the plan include a worker with the given filter?
    pub fn includes_filter(&self, interface_filter: Option<&str>) -> bool {
        self.workers
            .iter()
            .any(|w| w.interface_filter == interface_filter)
    }
}

/// Which `assistant`-binary role is starting up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryRole {
    /// `assistant` interactive REPL (no subcommand).
    Repl,
    /// `assistant orchestrator run` without `--interfaces`.
    OrchestratorUnfiltered,
    /// `assistant orchestrator run --interfaces a,b,c`.
    OrchestratorFiltered,
    /// `assistant slack | mattermost | nextcloud | signal | matrix` —
    /// single-interface daemon. The interface-specific worker is spawned
    /// alongside the adapter init code, not by this plan.
    InterfaceOnly,
    /// `assistant mcp` — stdio JSON-RPC server. No turn workers.
    Mcp,
    /// `assistant worker --interface X` — single dedicated worker. No
    /// companion services.
    WorkerOnly,
    /// `assistant webui serve` — HTTP/SSE/UI server only; the orchestrator
    /// service owns the worker pool (see #730).
    WebUi,
}

/// Build the worker plan for a given binary role.
pub fn core_worker_plan(role: BinaryRole) -> CoreWorkerPlan {
    match role {
        BinaryRole::WebUi | BinaryRole::Mcp | BinaryRole::WorkerOnly => CoreWorkerPlan {
            workers: vec![],
            spawn_title_generator: false,
            spawn_memory_indexer: false,
        },
        BinaryRole::Repl | BinaryRole::OrchestratorUnfiltered => CoreWorkerPlan {
            workers: vec![CoreWorkerSpec {
                worker_id: "main-worker",
                interface_filter: None,
            }],
            spawn_title_generator: true,
            spawn_memory_indexer: true,
        },
        BinaryRole::OrchestratorFiltered => CoreWorkerPlan {
            workers: vec![
                CoreWorkerSpec {
                    worker_id: "scheduler-worker",
                    interface_filter: Some("Scheduler"),
                },
                CoreWorkerSpec {
                    worker_id: "web-worker",
                    interface_filter: Some("Web"),
                },
            ],
            spawn_title_generator: true,
            spawn_memory_indexer: true,
        },
        BinaryRole::InterfaceOnly => CoreWorkerPlan {
            // Interface-specific worker (slack-worker / matrix-worker / …)
            // plus a Scheduler-filtered worker are spawned alongside the
            // adapter init code, not here. The plan still asks for the
            // shared companions.
            workers: vec![],
            spawn_title_generator: true,
            spawn_memory_indexer: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webui_plan_is_empty() {
        let plan = core_worker_plan(BinaryRole::WebUi);
        assert!(
            plan.is_empty(),
            "web-ui must not spawn any infrastructure workers (#730)"
        );
        assert!(plan.workers.is_empty());
        assert!(!plan.spawn_title_generator);
        assert!(!plan.spawn_memory_indexer);
    }

    #[test]
    fn webui_plan_omits_web_filtered_worker() {
        let plan = core_worker_plan(BinaryRole::WebUi);
        assert!(
            !plan.includes_filter(Some("Web")),
            "web-ui must NOT spawn a Web-filtered worker — the orchestrator owns it"
        );
    }

    #[test]
    fn mcp_plan_is_empty() {
        let plan = core_worker_plan(BinaryRole::Mcp);
        assert!(plan.is_empty(), "mcp mode runs no turn workers");
    }

    #[test]
    fn worker_only_plan_is_empty() {
        let plan = core_worker_plan(BinaryRole::WorkerOnly);
        assert!(
            plan.is_empty(),
            "worker-only mode is a single dedicated worker"
        );
    }

    #[test]
    fn repl_plan_spawns_main_worker() {
        let plan = core_worker_plan(BinaryRole::Repl);
        assert!(plan.includes_filter(None));
        assert_eq!(plan.workers.len(), 1);
        assert_eq!(plan.workers[0].worker_id, "main-worker");
        assert!(plan.spawn_title_generator);
        assert!(plan.spawn_memory_indexer);
    }

    #[test]
    fn orchestrator_unfiltered_plan_matches_repl() {
        assert_eq!(
            core_worker_plan(BinaryRole::OrchestratorUnfiltered),
            core_worker_plan(BinaryRole::Repl),
        );
    }

    #[test]
    fn orchestrator_filtered_plan_includes_scheduler_and_web() {
        let plan = core_worker_plan(BinaryRole::OrchestratorFiltered);
        assert!(
            plan.includes_filter(Some("Scheduler")),
            "filtered orchestrator must include scheduler-worker"
        );
        assert!(
            plan.includes_filter(Some("Web")),
            "filtered orchestrator must include web-worker (#730)"
        );
        assert!(
            !plan.includes_filter(None),
            "filtered orchestrator must NOT spawn an unfiltered worker (would double-consume)"
        );
        assert!(plan.spawn_title_generator);
        assert!(plan.spawn_memory_indexer);
    }

    #[test]
    fn interface_only_plan_has_companions_no_main_worker() {
        let plan = core_worker_plan(BinaryRole::InterfaceOnly);
        assert!(
            plan.workers.is_empty(),
            "interface-specific worker is spawned by adapter init, not by the plan"
        );
        assert!(plan.spawn_title_generator);
        assert!(plan.spawn_memory_indexer);
    }
}
