//! Event trigger adapter: queues runs for event-triggered workflows from completed bus messages.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::{MessageBus, MessageStatus, bus_messages};
use assistant_storage::{StorageLayer, WorkflowNodeKind, WorkflowStore as _, WorkflowTriggerKind};
use chrono::{DateTime, Utc};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Spawn event trigger adapter from completed bus events.
pub fn spawn_event_trigger_adapter(
    storage: Arc<StorageLayer>,
    bus: Arc<dyn MessageBus>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow event adapter started");
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut seen_order: VecDeque<Uuid> = VecDeque::new();
        let started_at = SystemClock.now();
        let mut warned_done_filter_fallback = false;

        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_event_triggers(
                &storage,
                bus.as_ref(),
                started_at,
                &mut seen,
                &mut seen_order,
                &mut warned_done_filter_fallback,
            )
            .await
            {
                error!(error = %err, "Workflow event adapter iteration failed");
            }
        }
    })
}

async fn process_event_triggers(
    storage: &StorageLayer,
    bus: &dyn MessageBus,
    started_at: DateTime<Utc>,
    seen: &mut HashSet<Uuid>,
    seen_order: &mut VecDeque<Uuid>,
    warned_done_filter_fallback: &mut bool,
) -> Result<()> {
    let store = storage.workflow_store();
    let workflows = store.list().await?;

    for topic in [
        bus_messages::topic::TURN_RESULT,
        bus_messages::topic::TOOL_RESULT,
        bus_messages::topic::AGENT_REPORT,
    ] {
        let mut messages = bus.list(topic, Some(MessageStatus::Done), 50).await?;
        if messages.is_empty() {
            let fallback = bus.list(topic, None, 50).await?;
            if !fallback.is_empty() {
                if !*warned_done_filter_fallback {
                    warn!(
                        "MessageBus did not return done-filtered events; using unfiltered fallback for workflow event triggers"
                    );
                    *warned_done_filter_fallback = true;
                }
                messages = fallback;
            }
        }
        for msg in messages {
            if msg.created_at < started_at || seen.contains(&msg.id) {
                continue;
            }

            seen.insert(msg.id);
            seen_order.push_back(msg.id);
            while seen_order.len() > 4096 {
                if let Some(old) = seen_order.pop_front() {
                    seen.remove(&old);
                }
            }

            for workflow in workflows.iter().filter(|w| w.active) {
                for node in workflow
                    .graph
                    .nodes
                    .iter()
                    .filter(|n| n.kind == WorkflowNodeKind::Trigger)
                {
                    let trigger_type = node
                        .config
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("manual");
                    if trigger_type != "event" {
                        continue;
                    }

                    let expected_topic = node
                        .config
                        .get("event")
                        .or_else(|| node.config.get("topic"))
                        .and_then(|v| v.as_str());
                    if let Some(expected_topic) = expected_topic
                        && expected_topic != msg.topic
                    {
                        continue;
                    }

                    let payload = serde_json::json!({
                        "trigger_node_id": node.id,
                        "event_topic": msg.topic,
                        "message_id": msg.id,
                        "conversation_id": msg.conversation_id,
                        "payload": msg.payload,
                    });
                    let run_id = store
                        .create_run(workflow.id, WorkflowTriggerKind::Event, &payload)
                        .await?;
                    info!(workflow_id = %workflow.id, node_id = %node.id, run_id = %run_id, event_topic = %topic, "Event trigger queued workflow run");
                }
            }
        }
    }

    Ok(())
}
