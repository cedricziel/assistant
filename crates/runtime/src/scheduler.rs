//! Background scheduler — polls for due scheduled tasks and dispatches them
//! through the message bus.  Also drives the heartbeat loop (reads
//! `HEARTBEAT.md` from the configured memory path).
//!
//! Tasks and heartbeats are published as [`TurnRequest`] messages on the bus.
//! The orchestrator's [`run_worker`](crate::Orchestrator::run_worker) loop
//! claims and processes them asynchronously.

use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use assistant_core::{bus_messages, strip_html_comments, topic, Interface, PublishRequest};
use assistant_storage::StorageLayer;
use chrono::Utc;
use cron::Schedule;
use opentelemetry::{
    trace::{Span as _, SpanKind},
    KeyValue,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;
use crate::otel_spans::{start_interface_root_context, traceparent_from_context};
use crate::webhook_dispatch;

/// How often the heartbeat prompt is run (30 minutes).
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30 * 60);

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

        let turn_req = bus_messages::TurnRequest {
            prompt: task.prompt.clone(),
            conversation_id,
            extension_tools: vec![],
            timestamp: Some(Utc::now()),
            traceparent: traceparent_from_context(&interface_cx),
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

    let raw = tokio::fs::read_to_string(heartbeat_path).await?;
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
    let turn_req = bus_messages::TurnRequest {
        prompt,
        conversation_id,
        extension_tools: vec![],
        timestamp: Some(Utc::now()),
        traceparent: traceparent_from_context(&interface_cx),
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

// -- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assistant_core::{topic, AssistantConfig, MessageBus};
    use assistant_storage::StorageLayer;
    use assistant_tool_executor::ToolExecutor;
    use chrono::{Duration, Timelike, Utc};
    use tempfile::NamedTempFile;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;
    use crate::orchestrator::Orchestrator;
    use assistant_llm::{LlmClient, LlmClientConfig, LlmProvider};
    use assistant_storage::registry::SkillRegistry;

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
                retry_config: assistant_llm::RetryConfig::disabled(),
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

    #[tokio::test]
    async fn heartbeat_skipped_when_file_missing() {
        let server = MockServer::start().await;

        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
        // Point heartbeat_path at a path that cannot exist.
        config.memory.heartbeat_path =
            Some("/tmp/assistant-test-no-such-heartbeat-file.md".to_string());

        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: server.uri(),
                timeout_secs: 10,
                retry_config: assistant_llm::RetryConfig::disabled(),
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

        run_heartbeat(&orch).await.unwrap();

        let bus = storage.message_bus();
        let msg = bus
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
        let (mut config, storage) = {
            let mut c = AssistantConfig::default();
            c.memory.enabled = false;
            let s = Arc::new(StorageLayer::new_in_memory().await.unwrap());
            (c, s)
        };

        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "<!-- just a comment -->").unwrap();
        config.memory.heartbeat_path = Some(tmp.path().to_string_lossy().into_owned());

        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: server.uri(),
                timeout_secs: 10,
                retry_config: assistant_llm::RetryConfig::disabled(),
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
            executor,
            registry,
            bus,
            &config,
        ));

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

        let mut config = AssistantConfig::default();
        config.memory.enabled = false;

        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "Check system health.").unwrap();
        config.memory.heartbeat_path = Some(tmp.path().to_string_lossy().into_owned());

        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: server.uri(),
                timeout_secs: 10,
                retry_config: assistant_llm::RetryConfig::disabled(),
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
}
