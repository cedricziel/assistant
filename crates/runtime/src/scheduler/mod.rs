//! Background scheduler — polls for due scheduled tasks and dispatches them
//! through the message bus.  Also drives the heartbeat loop (reads
//! `HEARTBEAT.md` from the configured memory path).
//!
//! Tasks and heartbeats are published as [`TurnRequest`] messages on the bus.
//! The orchestrator's [`run_worker`](crate::Orchestrator::run_worker) loop
//! claims and processes them asynchronously.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use tracing::{error, info};

use assistant_storage::StorageLayer;

use crate::orchestrator::Orchestrator;

mod heartbeat;
mod prune;
mod reap;
mod tasks;

// Re-export crate-visible helpers used by the `tests/` submodules and by
// the rest of the runtime. `compute_next_run` / `resolve_home_channel_tools`
// are only reached by tests today, so they're test-gated to avoid a
// production-build dead-code warning.
pub(crate) use heartbeat::run_heartbeat;
pub(crate) use prune::prune_conversation_events;
pub(crate) use reap::reap_stale_and_recover;
pub(crate) use tasks::run_due_tasks;
#[cfg(test)]
pub(crate) use tasks::{compute_next_run, resolve_home_channel_tools};

/// How often the heartbeat prompt is run (30 minutes).
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// How often expired conversation events are pruned (60 minutes).
const EVENT_PRUNE_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// `user_id` stamped on every bus message produced by the scheduler subsystem
/// (both scheduled tasks and heartbeats).
const SCHEDULER_USER_ID: &str = "scheduler";

// ── PeriodicTask ──────────────────────────────────────────────────────────────

/// Shared context passed to every [`PeriodicTask`] invocation.
pub(crate) struct SchedulerCtx {
    pub storage: Arc<StorageLayer>,
    pub orchestrator: Arc<Orchestrator>,
}

/// A unit of recurring work driven by the scheduler loop.
///
/// Each implementor is invoked at most once per `interval()`. The scheduler
/// polls every `spawn_scheduler(... , poll_interval)` and runs whichever
/// tasks are due.  Set `run_before_loop()` to `true` to fire once at startup
/// before the first poll (useful for crash-recovery tasks that should not
/// wait for the first tick).
#[async_trait]
pub(crate) trait PeriodicTask: Send + Sync {
    fn name(&self) -> &'static str;

    fn interval(&self) -> Duration;

    /// Whether this task should fire once immediately at scheduler startup,
    /// before the first `poll_interval` sleep. Default: `false` — task fires
    /// on the first tick (since `last_run` is initialised to "already
    /// elapsed").
    fn run_before_loop(&self) -> bool {
        false
    }

    async fn run(&self, ctx: &SchedulerCtx) -> Result<()>;
}

// ── Built-in tasks ────────────────────────────────────────────────────────────

struct ScheduledTasks {
    poll_interval: Duration,
}

#[async_trait]
impl PeriodicTask for ScheduledTasks {
    fn name(&self) -> &'static str {
        "scheduled-tasks"
    }
    fn interval(&self) -> Duration {
        self.poll_interval
    }
    async fn run(&self, ctx: &SchedulerCtx) -> Result<()> {
        run_due_tasks(&ctx.storage, &ctx.orchestrator).await
    }
}

struct Heartbeat;

#[async_trait]
impl PeriodicTask for Heartbeat {
    fn name(&self) -> &'static str {
        "heartbeat"
    }
    fn interval(&self) -> Duration {
        HEARTBEAT_INTERVAL
    }
    async fn run(&self, ctx: &SchedulerCtx) -> Result<()> {
        run_heartbeat(&ctx.orchestrator).await
    }
}

struct ConversationEventPrune;

#[async_trait]
impl PeriodicTask for ConversationEventPrune {
    fn name(&self) -> &'static str {
        "event-prune"
    }
    fn interval(&self) -> Duration {
        EVENT_PRUNE_INTERVAL
    }
    fn run_before_loop(&self) -> bool {
        true
    }
    async fn run(&self, ctx: &SchedulerCtx) -> Result<()> {
        prune_conversation_events(&ctx.storage).await;
        Ok(())
    }
}

struct ReapStale;

#[async_trait]
impl PeriodicTask for ReapStale {
    fn name(&self) -> &'static str {
        "reap-stale"
    }
    fn interval(&self) -> Duration {
        reap::REAP_STALE_INTERVAL
    }
    fn run_before_loop(&self) -> bool {
        true
    }
    async fn run(&self, ctx: &SchedulerCtx) -> Result<()> {
        reap_stale_and_recover(&ctx.storage, ctx.orchestrator.bus().as_ref()).await;
        Ok(())
    }
}

/// Spawn a background tokio task that:
/// 1. Checks for due scheduled tasks every `poll_interval`.
/// 2. Reads `HEARTBEAT.md` (from the configured memory path) as a prompt
///    dispatched through the message bus every 30 minutes (if the file exists
///    and is non-empty).
pub fn spawn_scheduler(
    storage: Arc<StorageLayer>,
    orchestrator: Arc<Orchestrator>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    let ctx = Arc::new(SchedulerCtx {
        storage,
        orchestrator,
    });
    let tasks: Vec<Box<dyn PeriodicTask>> = vec![
        Box::new(ScheduledTasks { poll_interval }),
        Box::new(Heartbeat),
        Box::new(ConversationEventPrune),
        Box::new(ReapStale),
    ];
    tokio::spawn(run_scheduler_loop(ctx, tasks, poll_interval))
}

/// Body of the scheduler tokio task. Split out so the loop is testable
/// without `spawn_scheduler`'s detached task semantics.
pub(crate) async fn run_scheduler_loop(
    ctx: Arc<SchedulerCtx>,
    tasks: Vec<Box<dyn PeriodicTask>>,
    poll_interval: Duration,
) {
    info!("Scheduler started (poll interval: {:?})", poll_interval);

    // Pre-loop: fire any task that opts into `run_before_loop` (currently
    // the crash-recovery tasks). Initialise `last_run` for every task to
    // "already elapsed" so all of them are due on the first tick.
    let mut last_run: Vec<Instant> = Vec::with_capacity(tasks.len());
    for task in &tasks {
        if task.run_before_loop()
            && let Err(e) = task.run(&ctx).await
        {
            error!(task = task.name(), error = %e, "Periodic task startup error");
        }
        last_run.push(
            Instant::now()
                .checked_sub(task.interval())
                .unwrap_or_else(Instant::now),
        );
    }

    loop {
        tokio::time::sleep(poll_interval).await;

        for (idx, task) in tasks.iter().enumerate() {
            if last_run[idx].elapsed() < task.interval() {
                continue;
            }
            if let Err(e) = task.run(&ctx).await {
                error!(task = task.name(), error = %e, "Periodic task error");
            }
            last_run[idx] = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests;
