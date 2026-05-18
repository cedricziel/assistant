//! Scheduled-task dispatcher: cron-driven user tasks, the shared
//! `TurnRequest` publisher used by both tasks and heartbeats, the
//! `schedule.trigger` webhook/bus fan-out, home-channel tool resolution,
//! and the cron parser.

use assistant_storage::PersonaStore as _;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use chrono::{self, Utc};
use cron::Schedule;
use opentelemetry::{
    KeyValue,
    trace::{Span as _, SpanKind},
};
use tracing::{error, info, warn};
use uuid::Uuid;

use assistant_core::{
    ChannelContent, ChannelMessage, ChannelUser, PublishRequest, bus_messages, topic,
    types::conversation::ChannelType, types::conversation::Interface,
};
use assistant_storage::{ScheduledTaskStore, StorageLayer};

use super::SCHEDULER_USER_ID;
use crate::orchestrator::Orchestrator;
use crate::otel_spans::{start_interface_root_context, traceparent_from_context};
use crate::webhook_dispatch;

/// Dispatch due scheduled tasks by publishing [`TurnRequest`] messages to the
/// bus.  The worker loop processes them asynchronously.
pub(crate) async fn run_due_tasks(
    storage: &StorageLayer,
    orchestrator: &Orchestrator,
) -> Result<()> {
    let now = orchestrator.clock.now();
    let task_store = storage.scheduled_task_store_for_agent(&orchestrator.agent_id);
    let due = task_store.due_tasks(now).await?;

    for task in due {
        info!(task_name = %task.name, "Dispatching scheduled task");

        let conversation_id = Uuid::new_v4();
        let interface_cx = start_interface_root_context(
            &Interface::Scheduler,
            "schedule.dispatch",
            Some(conversation_id),
        );

        // Fire schedule.trigger webhook + bus event before the TurnRequest so
        // external listeners see the cron fire even if the turn fails to
        // dispatch.
        publish_schedule_trigger(
            storage,
            orchestrator,
            &task,
            now,
            conversation_id,
            &interface_cx,
        )
        .await;

        let dispatched = match dispatch_scheduler_turn_request(
            storage,
            orchestrator,
            task.prompt.clone(),
            conversation_id,
            &interface_cx,
        )
        .await
        {
            Ok(()) => {
                info!(
                    task_name = %task.name,
                    conversation_id = %conversation_id,
                    "Scheduled task dispatched to bus"
                );
                true
            }
            Err(e) => {
                error!(
                    task_name = %task.name,
                    error = %e,
                    "Failed to dispatch scheduled task"
                );
                false
            }
        };

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

/// Fire the `schedule.trigger` webhook and publish the same payload as a
/// bus event so external listeners observe every cron fire. Both arms log
/// (but never propagate) their failures — the caller still tries to
/// dispatch the TurnRequest regardless.
async fn publish_schedule_trigger(
    storage: &StorageLayer,
    orchestrator: &Orchestrator,
    task: &assistant_storage::ScheduledTask,
    now: chrono::DateTime<Utc>,
    conversation_id: Uuid,
    interface_cx: &opentelemetry::Context,
) {
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

    let mut span = crate::otel_spans::start_bus_span(
        SpanKind::Producer,
        topic::SCHEDULE_TRIGGER,
        Some(conversation_id),
        interface_cx,
    );
    match orchestrator
        .bus()
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
            span.set_attribute(KeyValue::new("bus.message_id", message_id.to_string()));
            span.set_attribute(KeyValue::new("bus.status", "ok"));
        }
        Err(e) => {
            span.set_attribute(KeyValue::new("bus.status", "error"));
            span.set_attribute(KeyValue::new("error", true));
            span.set_attribute(KeyValue::new("error.message", e.to_string()));
            error!(task_name = %task.name, error = %e, "Failed to publish schedule.trigger event");
        }
    }
    span.end();
}

/// Build a [`TurnRequest`] for a scheduler-originated dispatch, publish it
/// to the bus with proper OTel spans, and register any home-channel
/// platform tools beforehand.
///
/// Shared entry point for [`run_due_tasks`] and the heartbeat dispatcher —
/// both resolve persona-owner identity, home-channel tools, build the same
/// envelope, and publish to [`topic::TURN_REQUEST`]. Returns `Ok(())` on
/// success; `Err` if envelope serialisation or publish fails (caller
/// decides whether to propagate or merely log).
pub(super) async fn dispatch_scheduler_turn_request(
    storage: &StorageLayer,
    orchestrator: &Orchestrator,
    prompt: String,
    conversation_id: Uuid,
    interface_cx: &opentelemetry::Context,
) -> Result<()> {
    // Resolve home-channel tools and register them before publishing so the
    // worker can find them when it claims the message.
    let home_tools = resolve_home_channel_tools(storage, orchestrator, conversation_id).await;
    if !home_tools.is_empty() {
        orchestrator
            .register_extensions(conversation_id, home_tools, vec![])
            .await;
    }

    // Resolve the persona's owner so scheduler-driven turns carry identity.
    let persona_owner = storage
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
        timestamp: Some(orchestrator.clock.now()),
        traceparent: traceparent_from_context(interface_cx),
        attachment_ids: vec![],
        user_id: persona_owner,
        org_id: None,
        space_id: None,
    };

    let mut span = crate::otel_spans::start_bus_span(
        SpanKind::Producer,
        topic::TURN_REQUEST,
        Some(conversation_id),
        interface_cx,
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
            span.set_attribute(KeyValue::new("bus.message_id", message_id.to_string()));
            span.set_attribute(KeyValue::new("bus.status", "ok"));
            span.end();
            Ok(())
        }
        Err(e) => {
            span.set_attribute(KeyValue::new("bus.status", "error"));
            span.set_attribute(KeyValue::new("error", true));
            span.set_attribute(KeyValue::new("error.message", e.to_string()));
            span.end();
            Err(e)
        }
    }
}

/// Resolve home-channel platform tools for a scheduler-originated turn.
///
/// Loads the active persona's `home_channel`, looks up the matching live
/// adapter in the registry, and returns the adapter's platform tools pre-bound
/// to the configured channel address.  Returns an empty vec (with a warning
/// log) when no home channel is configured or no matching adapter is running.
pub(crate) async fn resolve_home_channel_tools(
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
        timestamp: orchestrator.clock.now(),
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
