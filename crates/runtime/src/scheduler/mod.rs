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

// Re-export crate-visible helpers so the in-file `mod tests` (and the
// existing `super::*` imports inside it) continue to work without
// per-test path updates.
#[allow(unused_imports)]
pub(crate) use heartbeat::run_heartbeat;
#[allow(unused_imports)]
pub(crate) use prune::prune_conversation_events;
#[allow(unused_imports)]
pub(crate) use reap::reap_stale_and_recover;
#[allow(unused_imports)]
pub(crate) use tasks::{compute_next_run, resolve_home_channel_tools, run_due_tasks};

// Re-imported here so `mod tests`'s `use super::*;` keeps working.
#[cfg(test)]
#[allow(unused_imports)]
use assistant_core::bus_messages;
#[cfg(test)]
#[allow(unused_imports)]
use uuid::Uuid;

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
async fn run_scheduler_loop(
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

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assistant_core::{LlmProvider, MessageBus, topic, types::agent::AssistantConfig};
    use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
    use assistant_storage::StorageLayer;
    use assistant_storage::registry::SkillRegistry;
    use assistant_tool_executor::ToolExecutor;
    use chrono::{Duration, Timelike, Utc};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;
    use crate::orchestrator::Orchestrator;

    // -- Helpers -------------------------------------------------------------

    fn ollama_answer(text: &str) -> serde_json::Value {
        serde_json::json!({
            "model": "test",
            "message": { "role": "assistant", "content": text },
            "done": true
        })
    }

    async fn mount_answer(server: &MockServer, text: &str) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer(text)))
            .mount(server)
            .await;
    }

    async fn build(base_url: &str) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: base_url.to_string(),
                timeout_secs: 10,
                retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
            })
            .unwrap(),
        );
        let executor = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        let orch = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        ));
        executor.set_subagent_runner(orch.clone());
        (orch, storage)
    }

    // -- compute_next_run ----------------------------------------------------

    #[test]
    fn next_run_seven_field_cron() {
        // "0 0 9 * * *" — 7-field, fires at 09:00 every day
        let next = compute_next_run("0 0 9 * * *");
        assert!(next.is_some(), "should parse 7-field cron");
        let next = next.unwrap();
        assert!(next > Utc::now(), "next run must be in the future");
        assert_eq!(next.time().hour(), 9, "should fire at 09:xx");
    }

    #[test]
    fn next_run_five_field_cron() {
        // "0 9 * * *" — standard 5-field, also fires at 09:00
        let next = compute_next_run("0 9 * * *");
        assert!(next.is_some(), "should parse 5-field cron");
        let next = next.unwrap();
        assert!(next > Utc::now(), "next run must be in the future");
        assert_eq!(next.time().hour(), 9, "should fire at 09:xx");
    }

    #[test]
    fn next_run_invalid_expr_returns_none() {
        assert!(compute_next_run("not a cron").is_none());
    }

    // -- run_due_tasks -------------------------------------------------------

    #[tokio::test]
    async fn due_task_is_published_to_bus() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (orch, storage) = build(&server.uri()).await;

        let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
        let past = Utc::now() - Duration::seconds(60);
        store
            .insert("test-task", "0 0 * * *", "say hello", false, Some(past))
            .await
            .unwrap();

        run_due_tasks(&storage, &orch).await.unwrap();

        let bus = storage.message_bus();
        let msg = bus
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap();
        assert!(msg.is_some(), "turn.request should be on the bus");
        let msg = msg.unwrap();
        assert_eq!(
            msg.interface.as_deref(),
            Some("Scheduler"),
            "interface must be Scheduler"
        );
        assert_eq!(
            msg.user_id.as_deref(),
            Some(SCHEDULER_USER_ID),
            "user_id must be the scheduler constant"
        );
    }

    #[tokio::test]
    async fn once_task_is_disabled_after_dispatch() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (orch, storage) = build(&server.uri()).await;

        let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
        let past = Utc::now() - Duration::seconds(60);
        let id = store
            .insert("once-task", "", "ping", true, Some(past))
            .await
            .unwrap();

        run_due_tasks(&storage, &orch).await.unwrap();

        let tasks = store.list_all().await.unwrap();
        let task = tasks.iter().find(|t| t.id == id).unwrap();
        assert!(
            !task.enabled,
            "one-shot task must be disabled after dispatch"
        );
        assert!(task.last_run.is_some(), "last_run must be recorded");
    }

    #[tokio::test]
    async fn recurring_task_next_run_is_advanced() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (orch, storage) = build(&server.uri()).await;

        let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
        let past = Utc::now() - Duration::seconds(60);
        let id = store
            .insert("cron-task", "0 0 9 * * *", "morning", false, Some(past))
            .await
            .unwrap();

        run_due_tasks(&storage, &orch).await.unwrap();

        let tasks = store.list_all().await.unwrap();
        let task = tasks.iter().find(|t| t.id == id).unwrap();
        assert!(task.enabled, "recurring task must stay enabled");
        assert!(
            task.next_run.is_some_and(|nr| nr > Utc::now()),
            "next_run must be advanced into the future"
        );
    }

    #[tokio::test]
    async fn not_due_task_is_not_dispatched() {
        let server = MockServer::start().await;
        let (orch, storage) = build(&server.uri()).await;

        let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
        let future = Utc::now() + Duration::hours(1);
        store
            .insert("future-task", "0 0 9 * * *", "later", false, Some(future))
            .await
            .unwrap();

        run_due_tasks(&storage, &orch).await.unwrap();

        let bus = storage.message_bus();
        let msg = bus
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap();
        assert!(
            msg.is_none(),
            "no turn.request should be published for a future task"
        );
    }

    // -- run_heartbeat -------------------------------------------------------

    /// RAII guard that removes the agent directory when dropped.
    struct AgentDirGuard(std::path::PathBuf);
    impl Drop for AgentDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    impl std::ops::Deref for AgentDirGuard {
        type Target = std::path::Path;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    /// Build an orchestrator whose agent_id maps to `agent_dir` so tests can
    /// control the HEARTBEAT.md content without touching ~/.assistant.
    ///
    /// `heartbeat_path()` returns `agent_base_dir(agent_id).join("HEARTBEAT.md")`,
    /// i.e. `~/.assistant/agents/{agent_id}/HEARTBEAT.md`.  We use a unique
    /// agent_id (UUID) so different tests don't collide.  The returned
    /// `AgentDirGuard` removes the directory automatically when dropped.
    async fn build_with_agent_dir(
        base_url: &str,
    ) -> (Arc<Orchestrator>, Arc<StorageLayer>, AgentDirGuard, String) {
        let agent_id = Uuid::new_v4().to_string();
        let agent_dir = assistant_core::context::agent_base_dir(&agent_id);
        std::fs::create_dir_all(&agent_dir).unwrap();

        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
        config.agent.id = agent_id.clone();

        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: base_url.to_string(),
                timeout_secs: 10,
                retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
            })
            .unwrap(),
        );
        let executor = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        let bus_arc: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        let orch = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            executor,
            registry,
            bus_arc,
            &config,
        ));
        (orch, storage, AgentDirGuard(agent_dir), agent_id)
    }

    #[tokio::test]
    async fn heartbeat_skipped_when_file_missing() {
        let server = MockServer::start().await;
        // agent dir is created but we deliberately don't write HEARTBEAT.md
        let (orch, storage, _agent_dir, _agent_id) = build_with_agent_dir(&server.uri()).await;

        run_heartbeat(&orch).await.unwrap();

        let msg = storage
            .message_bus()
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap();
        assert!(
            msg.is_none(),
            "no message should be published when HEARTBEAT.md is absent"
        );
    }

    #[tokio::test]
    async fn heartbeat_skipped_when_file_empty_after_comment_strip() {
        let server = MockServer::start().await;
        let (orch, storage, agent_dir, _agent_id) = build_with_agent_dir(&server.uri()).await;

        std::fs::write(agent_dir.join("HEARTBEAT.md"), "<!-- just a comment -->").unwrap();

        run_heartbeat(&orch).await.unwrap();

        let msg = storage
            .message_bus()
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap();
        assert!(
            msg.is_none(),
            "comment-only HEARTBEAT.md must produce no message"
        );
    }

    #[tokio::test]
    async fn heartbeat_publishes_turn_request_with_scheduler_user_id() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;

        let (orch, storage, agent_dir, _agent_id) = build_with_agent_dir(&server.uri()).await;

        std::fs::write(agent_dir.join("HEARTBEAT.md"), "Check system health.").unwrap();

        run_heartbeat(&orch).await.unwrap();

        let msg = storage
            .message_bus()
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap();
        assert!(
            msg.is_some(),
            "HEARTBEAT.md with content must publish a turn.request"
        );
        let msg = msg.unwrap();
        assert_eq!(
            msg.interface.as_deref(),
            Some("Scheduler"),
            "heartbeat must use Scheduler interface"
        );
        assert_eq!(
            msg.user_id.as_deref(),
            Some(SCHEDULER_USER_ID),
            "heartbeat must use SCHEDULER_USER_ID constant"
        );

        // Verify the prompt text was preserved
        let payload: bus_messages::TurnRequest = serde_json::from_value(msg.payload).unwrap();
        assert_eq!(payload.prompt.trim(), "Check system health.");
    }

    // -- home channel resolution -----------------------------------------------

    #[tokio::test]
    async fn resolve_home_channel_tools_no_home_channel_returns_empty() {
        let server = MockServer::start().await;
        let (orch, storage) = build(&server.uri()).await;

        // Persona exists but has no home_channel set (default).
        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();

        let conv_id = Uuid::new_v4();
        let tools = resolve_home_channel_tools(&storage, &orch, conv_id).await;
        assert!(
            tools.is_empty(),
            "no home channel configured — should return empty tools"
        );
    }

    #[tokio::test]
    async fn resolve_home_channel_tools_adapter_not_registered_returns_empty() {
        let server = MockServer::start().await;
        let (orch, storage) = build(&server.uri()).await;

        storage
            .persona_store()
            .ensure_exists("default")
            .await
            .unwrap();
        storage
            .persona_store()
            .set_home_channel("default", "slack", "#ops")
            .await
            .unwrap();

        // No adapter registered — should degrade gracefully.
        let conv_id = Uuid::new_v4();
        let tools = resolve_home_channel_tools(&storage, &orch, conv_id).await;
        assert!(
            tools.is_empty(),
            "no adapter running — should return empty tools"
        );
    }

    #[tokio::test]
    async fn due_task_fires_without_output_tools_when_no_home_channel() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (orch, storage) = build(&server.uri()).await;

        let store = storage.scheduled_task_store_for_agent(&orch.agent_id);
        let past = Utc::now() - Duration::seconds(60);
        store
            .insert("no-home-task", "0 0 * * *", "ping", false, Some(past))
            .await
            .unwrap();

        // Task fires normally even without home_channel.
        run_due_tasks(&storage, &orch).await.unwrap();

        let bus = storage.message_bus();
        let msg = bus
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap();
        assert!(msg.is_some(), "turn.request must be published");
    }

    // -- reap_stale_and_recover ------------------------------------------------

    #[tokio::test]
    async fn reap_stale_closes_orphaned_run() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (_orch, storage) = build(&server.uri()).await;

        let event_store = storage.conversation_event_store();
        let run_id = "run-orphan-001";
        let conv_id = "conv-orphan-001";

        // Simulate an orphaned run: run_started but no terminal event.
        event_store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();
        event_store
            .append_event(
                run_id,
                conv_id,
                1,
                "token",
                &serde_json::json!({"token": "partial"}),
            )
            .await
            .unwrap();

        // Backdate so it's older than the stale threshold.
        sqlx::query(
            "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
             WHERE run_id = ?1",
        )
        .bind(run_id)
        .execute(&storage.pool)
        .await
        .unwrap();

        let bus = storage.message_bus();
        reap_stale_and_recover(&storage, &bus).await;

        // The orphaned run should now have a synthetic terminal event.
        assert!(
            event_store.is_run_complete(run_id).await.unwrap(),
            "orphaned run should be marked complete after recovery"
        );

        let events = event_store.list_events_since(run_id, 2).await.unwrap();
        assert_eq!(events.len(), 1, "should have exactly one synthetic event");
        assert_eq!(events[0].event_type, "agent_error");
        assert_eq!(events[0].payload["synthetic"], true);
    }

    #[tokio::test]
    async fn reap_stale_skips_run_with_active_bus_message() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (_orch, storage) = build(&server.uri()).await;

        let event_store = storage.conversation_event_store();
        let run_id = "run-active-001";
        let conv_id = Uuid::new_v4();

        // Create an orphaned run.
        event_store
            .append_event(
                run_id,
                &conv_id.to_string(),
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();

        // Backdate.
        sqlx::query(
            "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
             WHERE run_id = ?1",
        )
        .bind(run_id)
        .execute(&storage.pool)
        .await
        .unwrap();

        // Publish an active bus message for the same conversation.
        let bus = storage.message_bus();
        use assistant_core::PublishRequest;
        bus.publish(
            PublishRequest::new(topic::TURN_REQUEST, serde_json::json!({"prompt": "retry"}))
                .with_conversation_id(conv_id),
        )
        .await
        .unwrap();

        reap_stale_and_recover(&storage, &bus).await;

        // The run should NOT be closed — bus message is still active.
        assert!(
            !event_store.is_run_complete(run_id).await.unwrap(),
            "run with active bus message should not be closed"
        );
    }

    #[tokio::test]
    async fn reap_stale_skips_fresh_runs() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (_orch, storage) = build(&server.uri()).await;

        let event_store = storage.conversation_event_store();
        let run_id = "run-fresh-001";
        let conv_id = "conv-fresh-001";

        // Create a run that just started (no backdating).
        event_store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();

        let bus = storage.message_bus();
        reap_stale_and_recover(&storage, &bus).await;

        // Fresh run should not be touched.
        assert!(
            !event_store.is_run_complete(run_id).await.unwrap(),
            "fresh run should not be closed"
        );
    }

    #[tokio::test]
    async fn reap_stale_recovers_despite_non_turn_request_bus_message() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (_orch, storage) = build(&server.uri()).await;

        let event_store = storage.conversation_event_store();
        let run_id = "run-nonturn-001";
        let conv_id = Uuid::new_v4();

        // Create an orphaned run.
        event_store
            .append_event(
                run_id,
                &conv_id.to_string(),
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();

        // Backdate.
        sqlx::query(
            "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
             WHERE run_id = ?1",
        )
        .bind(run_id)
        .execute(&storage.pool)
        .await
        .unwrap();

        // Publish a non-turn.request bus message on the same conversation.
        // This should NOT block recovery — only turn.request matters.
        let bus = storage.message_bus();
        use assistant_core::PublishRequest;
        bus.publish(
            PublishRequest::new(
                topic::SCHEDULE_TRIGGER,
                serde_json::json!({"task_name": "irrelevant"}),
            )
            .with_conversation_id(conv_id),
        )
        .await
        .unwrap();

        reap_stale_and_recover(&storage, &bus).await;

        // The orphan should be recovered despite the schedule.trigger message.
        assert!(
            event_store.is_run_complete(run_id).await.unwrap(),
            "non-turn.request bus message should not block orphan recovery"
        );
    }

    // -- persona identity in scheduled tasks ------------------------------------

    #[tokio::test]
    async fn scheduled_task_carries_persona_owner_identity() {
        let server = MockServer::start().await;
        mount_answer(&server, "done").await;
        let (orch, storage) = build(&server.uri()).await;

        // Create a persona with an owner and set it as the orchestrator's agent.
        let persona_store = storage.persona_store();
        persona_store
            .create_owned("test-persona", "Test Persona", "usr_alice")
            .await
            .unwrap();
        // Override the orchestrator's agent_id to match the persona.
        // Safety: we need interior mutability for this test — use unsafe to
        // mutate through the Arc since we hold the only reference.
        unsafe {
            let orch_mut = Arc::as_ptr(&orch) as *mut Orchestrator;
            (*orch_mut).agent_id = "test-persona".to_string();
        }

        let store = storage.scheduled_task_store_for_agent("test-persona");
        let past = Utc::now() - Duration::seconds(60);
        store
            .insert(
                "identity-task",
                "0 0 * * *",
                "probe identity",
                false,
                Some(past),
            )
            .await
            .unwrap();

        run_due_tasks(&storage, &orch).await.unwrap();

        let bus = storage.message_bus();
        let msg = bus
            .claim(topic::TURN_REQUEST, "test-consumer")
            .await
            .unwrap()
            .expect("turn.request should be on the bus");

        let turn_req: bus_messages::TurnRequest = serde_json::from_value(msg.payload).unwrap();

        assert_eq!(
            turn_req.user_id.as_deref(),
            Some("usr_alice"),
            "TurnRequest should carry the persona owner's user_id"
        );
    }
}
