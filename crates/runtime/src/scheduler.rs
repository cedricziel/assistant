//! Background scheduler — polls for due scheduled tasks and dispatches them
//! through the message bus.  Also drives the heartbeat loop (reads
//! `HEARTBEAT.md` from the configured memory path).
//!
//! Tasks and heartbeats are published as [`TurnRequest`] messages on the bus.
//! The orchestrator's [`run_worker`](crate::Orchestrator::run_worker) loop
//! claims and processes them asynchronously.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use assistant_core::{
    ChannelContent, ChannelMessage, ChannelType, ChannelUser, Interface, MessageBus,
    PublishRequest, bus_messages, strip_html_comments, topic,
};
use assistant_storage::StorageLayer;
use chrono::Utc;
use cron::Schedule;
use opentelemetry::{
    KeyValue,
    trace::{Span as _, SpanKind},
};
use sqlx::SqlitePool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;
use crate::otel_spans::{start_interface_root_context, traceparent_from_context};
use crate::webhook_dispatch;

/// How often the heartbeat prompt is run (30 minutes).
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// How often expired conversation events are pruned (60 minutes).
const EVENT_PRUNE_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// `user_id` stamped on every bus message produced by the scheduler subsystem
/// (both scheduled tasks and heartbeats).
const SCHEDULER_USER_ID: &str = "scheduler";

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
    tokio::spawn(async move {
        info!("Scheduler started (poll interval: {:?})", poll_interval);
        // Subtract the full interval so the heartbeat fires on the first tick.
        let mut last_heartbeat = Instant::now()
            .checked_sub(HEARTBEAT_INTERVAL)
            .unwrap_or_else(Instant::now);
        let mut last_event_prune = Instant::now()
            .checked_sub(EVENT_PRUNE_INTERVAL)
            .unwrap_or_else(Instant::now);
        let mut last_reap_stale = Instant::now()
            .checked_sub(REAP_STALE_INTERVAL)
            .unwrap_or_else(Instant::now);

        // Run once on startup.
        prune_conversation_events(&storage).await;
        reap_stale_and_recover(&storage, orchestrator.bus().as_ref()).await;

        loop {
            tokio::time::sleep(poll_interval).await;

            if let Err(e) = run_due_tasks(&storage, &orchestrator).await {
                error!("Scheduler error: {e}");
            }

            if last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL {
                match run_heartbeat(&orchestrator).await {
                    Ok(()) => last_heartbeat = Instant::now(),
                    Err(e) => error!("Heartbeat error: {e}"),
                }
            }

            if last_event_prune.elapsed() >= EVENT_PRUNE_INTERVAL {
                prune_conversation_events(&storage).await;
                last_event_prune = Instant::now();
            }

            if last_reap_stale.elapsed() >= REAP_STALE_INTERVAL {
                reap_stale_and_recover(&storage, orchestrator.bus().as_ref()).await;
                last_reap_stale = Instant::now();
            }
        }
    })
}

/// Dispatch due scheduled tasks by publishing [`TurnRequest`] messages to the
/// bus.  The worker loop processes them asynchronously.
pub(crate) async fn run_due_tasks(
    storage: &StorageLayer,
    orchestrator: &Orchestrator,
) -> Result<()> {
    let now = Utc::now();
    let task_store = storage.scheduled_task_store_for_agent(&orchestrator.agent_id);
    let due = task_store.due_tasks(now).await?;
    let bus = orchestrator.bus();

    // Resolve the persona's owner so scheduled tasks carry identity context.
    let persona_owner = storage
        .persona_store()
        .get(&orchestrator.agent_id)
        .await
        .ok()
        .flatten()
        .and_then(|p| p.owner_user_id);

    for task in due {
        info!(task_name = %task.name, "Dispatching scheduled task");

        let conversation_id = Uuid::new_v4();
        let interface_cx = start_interface_root_context(
            &Interface::Scheduler,
            "schedule.dispatch",
            Some(conversation_id),
        );

        let trigger = serde_json::json!({
            "task_id": task.id,
            "task_name": task.name.clone(),
            "conversation_id": conversation_id,
            "triggered_at": now,
        });
        if let Err(e) = webhook_dispatch::dispatch_event(
            storage,
            &orchestrator.agent_id,
            topic::SCHEDULE_TRIGGER,
            trigger.clone(),
        )
        .await
        {
            error!(task_name = %task.name, error = %e, "Failed to dispatch schedule.trigger webhooks");
        }
        let mut produce_trigger_span = crate::otel_spans::start_bus_span(
            SpanKind::Producer,
            topic::SCHEDULE_TRIGGER,
            Some(conversation_id),
            &interface_cx,
        );
        match bus
            .publish(
                PublishRequest::new(topic::SCHEDULE_TRIGGER, trigger)
                    .with_agent_id(orchestrator.agent_id.clone())
                    .with_conversation_id(conversation_id)
                    .with_interface(format!("{:?}", Interface::Scheduler))
                    .with_user_id(SCHEDULER_USER_ID),
            )
            .await
        {
            Ok(message_id) => {
                produce_trigger_span
                    .set_attribute(KeyValue::new("bus.message_id", message_id.to_string()));
                produce_trigger_span.set_attribute(KeyValue::new("bus.status", "ok"));
            }
            Err(e) => {
                produce_trigger_span.set_attribute(KeyValue::new("bus.status", "error"));
                produce_trigger_span.set_attribute(KeyValue::new("error", true));
                produce_trigger_span.set_attribute(KeyValue::new("error.message", e.to_string()));
                error!(task_name = %task.name, error = %e, "Failed to publish schedule.trigger event");
            }
        }
        produce_trigger_span.end();

        // Resolve home-channel tools and register them before publishing the
        // TurnRequest so the worker can find them when it claims the message.
        let home_tools = resolve_home_channel_tools(storage, orchestrator, conversation_id).await;
        if !home_tools.is_empty() {
            orchestrator
                .register_extensions(conversation_id, home_tools, vec![])
                .await;
        }

        let turn_req = bus_messages::TurnRequest {
            prompt: task.prompt.clone(),
            conversation_id,
            extension_tools: vec![],
            timestamp: Some(Utc::now()),
            traceparent: traceparent_from_context(&interface_cx),
            attachment_ids: vec![],
            user_id: persona_owner.clone(),
            org_id: None,
            space_id: None,
        };

        let mut produce_turn_span = crate::otel_spans::start_bus_span(
            SpanKind::Producer,
            topic::TURN_REQUEST,
            Some(conversation_id),
            &interface_cx,
        );
        let dispatched = match bus
            .publish(
                PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                    .with_agent_id(orchestrator.agent_id.clone())
                    .with_conversation_id(conversation_id)
                    .with_interface(format!("{:?}", Interface::Scheduler))
                    .with_user_id(SCHEDULER_USER_ID),
            )
            .await
        {
            Ok(message_id) => {
                produce_turn_span
                    .set_attribute(KeyValue::new("bus.message_id", message_id.to_string()));
                produce_turn_span.set_attribute(KeyValue::new("bus.status", "ok"));
                info!(
                    task_name = %task.name,
                    conversation_id = %conversation_id,
                    "Scheduled task dispatched to bus"
                );
                true
            }
            Err(e) => {
                produce_turn_span.set_attribute(KeyValue::new("bus.status", "error"));
                produce_turn_span.set_attribute(KeyValue::new("error", true));
                produce_turn_span.set_attribute(KeyValue::new("error.message", e.to_string()));
                error!(
                    task_name = %task.name,
                    error = %e,
                    "Failed to dispatch scheduled task"
                );
                false
            }
        };
        produce_turn_span.end();

        if !dispatched {
            continue;
        }

        if task.once {
            // One-shot task: record the run and disable it.
            task_store.record_run(task.id, now, None).await?;
            task_store.disable(task.id).await?;
            info!(task_name = %task.name, "One-shot task disabled after dispatch");
        } else {
            // Recurring task: compute the next run from the cron expression.
            let next_run = compute_next_run(&task.cron_expr);
            task_store.record_run(task.id, now, next_run).await?;
        }
    }

    Ok(())
}

/// Resolve home-channel platform tools for a scheduler-originated turn.
///
/// Loads the active persona's `home_channel`, looks up the matching live
/// adapter in the registry, and returns the adapter's platform tools pre-bound
/// to the configured channel address.  Returns an empty vec (with a warning
/// log) when no home channel is configured or no matching adapter is running.
async fn resolve_home_channel_tools(
    storage: &StorageLayer,
    orchestrator: &Orchestrator,
    conversation_id: Uuid,
) -> Vec<Arc<dyn assistant_core::ToolHandler>> {
    let persona_store = storage.persona_store();
    let persona = match persona_store.get(&orchestrator.agent_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return vec![],
        Err(e) => {
            warn!(error = %e, "Failed to load persona for home channel resolution");
            return vec![];
        }
    };

    let hc = match &persona.home_channel {
        Some(hc) => hc.clone(),
        None => return vec![],
    };

    let adapter = match orchestrator.adapter_registry.get(&hc.home_interface).await {
        Some(a) => a,
        None => {
            warn!(
                interface = %hc.home_interface,
                channel = %hc.home_channel,
                "No live adapter found for home_interface — scheduler turn has no output tools"
            );
            return vec![];
        }
    };

    let channel_type = match hc.home_interface.as_str() {
        "slack" => ChannelType::Slack,
        "mattermost" => ChannelType::Mattermost,
        "matrix" => ChannelType::Matrix,
        "nextcloud" => ChannelType::Nextcloud,
        "signal" => ChannelType::Signal,
        other => ChannelType::Custom(other.to_string()),
    };

    let synthetic_msg = ChannelMessage {
        channel_type,
        platform_message_id: None,
        sender: ChannelUser {
            platform_id: hc.home_channel.clone(),
            display_name: None,
        },
        content: ChannelContent::Text(String::new()),
        thread_id: None,
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    };

    adapter.platform_tools(&synthetic_msg, conversation_id)
}

/// Compute the next occurrence after now for a cron expression.
/// Accepts both 5-field (standard) and 7-field (with seconds) expressions.
pub(crate) fn compute_next_run(cron_expr: &str) -> Option<chrono::DateTime<Utc>> {
    let schedule = Schedule::from_str(cron_expr)
        .or_else(|_| Schedule::from_str(&format!("0 {}", cron_expr)))
        .ok()?;
    schedule.upcoming(Utc).next()
}

/// Read `HEARTBEAT.md` (from the configured path) and dispatch its contents
/// as a [`TurnRequest`] through the message bus.
///
/// Does nothing (silently) if the file does not exist or is empty.
pub(crate) async fn run_heartbeat(orchestrator: &Orchestrator) -> Result<()> {
    let heartbeat_path = orchestrator.heartbeat_path();

    if !heartbeat_path.exists() {
        return Ok(());
    }

    let raw = tokio::fs::read_to_string(&heartbeat_path).await?;
    let prompt = strip_html_comments(&raw);

    if prompt.is_empty() {
        return Ok(());
    }

    info!("Dispatching heartbeat from {}", heartbeat_path.display());

    let conversation_id = Uuid::new_v4();
    let interface_cx = start_interface_root_context(
        &Interface::Scheduler,
        "heartbeat.dispatch",
        Some(conversation_id),
    );

    // Resolve home-channel tools and register them before publishing.
    let storage = orchestrator.storage.clone();
    let home_tools = resolve_home_channel_tools(&storage, orchestrator, conversation_id).await;
    if !home_tools.is_empty() {
        orchestrator
            .register_extensions(conversation_id, home_tools, vec![])
            .await;
    }

    // Resolve persona owner for identity context.
    let persona_owner = orchestrator
        .storage
        .persona_store()
        .get(&orchestrator.agent_id)
        .await
        .ok()
        .flatten()
        .and_then(|p| p.owner_user_id);

    let turn_req = bus_messages::TurnRequest {
        prompt,
        conversation_id,
        extension_tools: vec![],
        timestamp: Some(Utc::now()),
        traceparent: traceparent_from_context(&interface_cx),
        attachment_ids: vec![],
        user_id: persona_owner,
        org_id: None,
        space_id: None,
    };

    let mut produce_turn_span = crate::otel_spans::start_bus_span(
        SpanKind::Producer,
        topic::TURN_REQUEST,
        Some(conversation_id),
        &interface_cx,
    );
    let publish_result = orchestrator
        .bus()
        .publish(
            PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                .with_agent_id(orchestrator.agent_id.clone())
                .with_conversation_id(conversation_id)
                .with_interface(format!("{:?}", Interface::Scheduler))
                .with_user_id(SCHEDULER_USER_ID),
        )
        .await;
    match publish_result {
        Ok(message_id) => {
            produce_turn_span
                .set_attribute(KeyValue::new("bus.message_id", message_id.to_string()));
            produce_turn_span.set_attribute(KeyValue::new("bus.status", "ok"));
            produce_turn_span.end();
        }
        Err(e) => {
            produce_turn_span.set_attribute(KeyValue::new("bus.status", "error"));
            produce_turn_span.set_attribute(KeyValue::new("error", true));
            produce_turn_span.set_attribute(KeyValue::new("error.message", e.to_string()));
            produce_turn_span.end();
            return Err(e);
        }
    }

    info!(
        conversation_id = %conversation_id,
        "Heartbeat dispatched to bus"
    );

    Ok(())
}

// -- Conversation event pruning --------------------------------------------

/// Delete expired conversation events. Logs the result at debug level.
async fn prune_conversation_events(storage: &StorageLayer) {
    let store = storage.conversation_event_store();
    match store.prune_expired().await {
        Ok(n) if n > 0 => info!("Pruned {n} expired conversation event(s)"),
        Ok(_) => {}
        Err(e) => warn!("Failed to prune conversation events: {e}"),
    }
}

// -- Crash recovery ---------------------------------------------------------

/// How often stale bus messages and orphaned SSE runs are checked (5 minutes).
const REAP_STALE_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// A claimed bus message older than this is considered stale (5 minutes).
const STALE_CLAIM_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Reclaim stale bus messages and close orphaned SSE event streams.
///
/// 1. Calls `bus.reap_stale()` to reset stale `Claimed` → `Pending` messages.
/// 2. Finds runs that have a `run_started` event but no terminal event and are
///    older than `STALE_CLAIM_TIMEOUT`.
/// 3. For each orphan, checks whether an active (Pending/Claimed) bus message
///    still references that conversation — if so, the run may still complete
///    after bus redelivery, so we skip it.
/// 4. Otherwise, appends a synthetic `agent_error` event so clients see a
///    terminal event and can retry cleanly.
pub(crate) async fn reap_stale_and_recover(storage: &StorageLayer, bus: &dyn MessageBus) {
    // Step 1: reclaim stale bus messages.
    match bus.reap_stale(STALE_CLAIM_TIMEOUT).await {
        Ok(n) if n > 0 => info!("Reaped {n} stale bus message(s)"),
        Ok(_) => {}
        Err(e) => warn!("Failed to reap stale bus messages: {e}"),
    }

    // Step 2: find orphaned SSE runs.
    let event_store = storage.conversation_event_store();
    let stale_timeout = chrono::Duration::from_std(STALE_CLAIM_TIMEOUT).unwrap_or_default();
    let orphans = match event_store.find_incomplete_runs(stale_timeout).await {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to find incomplete runs: {e}");
            return;
        }
    };

    if orphans.is_empty() {
        return;
    }

    info!(
        count = orphans.len(),
        "Found orphaned SSE run(s), checking for active bus messages"
    );

    // Step 3 & 4: for each orphan, check bus and potentially close.
    for (run_id, conversation_id) in orphans {
        let has_active = match has_active_bus_message(&storage.pool, &conversation_id).await {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    run_id = %run_id,
                    error = %e,
                    "Failed to check bus messages for orphan — skipping"
                );
                continue;
            }
        };

        if has_active {
            info!(
                run_id = %run_id,
                conversation_id = %conversation_id,
                "Orphaned run still has active bus message — skipping (may recover)"
            );
            continue;
        }

        match event_store
            .append_synthetic_terminal(
                &run_id,
                &conversation_id,
                "Server restarted — this run was interrupted. You may retry your message.",
            )
            .await
        {
            Ok(seq) => info!(
                run_id = %run_id,
                conversation_id = %conversation_id,
                sequence = seq,
                "Appended synthetic agent_error to orphaned run"
            ),
            Err(e) => warn!(
                run_id = %run_id,
                error = %e,
                "Failed to append synthetic terminal for orphaned run"
            ),
        }
    }
}

/// Check whether there is a Pending or Claimed `turn.request` bus message for a
/// conversation.  Only `turn.request` messages initiate orchestrator runs; other
/// topics (e.g. `schedule.trigger`) should not block orphan recovery.
async fn has_active_bus_message(pool: &SqlitePool, conversation_id: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bus_messages \
         WHERE conversation_id = ?1 \
           AND topic = 'turn.request' \
           AND status IN ('pending', 'claimed')",
    )
    .bind(conversation_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assistant_core::{AssistantConfig, LlmProvider, MessageBus, topic};
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
            task.next_run.map_or(false, |nr| nr > Utc::now()),
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
        storage.persona_store().ensure_default().await.unwrap();

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

        storage.persona_store().ensure_default().await.unwrap();
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
