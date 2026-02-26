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
use tracing::{error, info};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;

/// How often the heartbeat prompt is run (30 minutes).
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30 * 60);

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
async fn run_due_tasks(storage: &StorageLayer, orchestrator: &Orchestrator) -> Result<()> {
    let now = Utc::now();
    let task_store = storage.scheduled_task_store();
    let due = task_store.due_tasks(now).await?;
    let bus = orchestrator.bus();

    for task in due {
        info!(task_name = %task.name, "Dispatching scheduled task");

        let conversation_id = Uuid::new_v4();
        let turn_req = bus_messages::TurnRequest {
            prompt: task.prompt.clone(),
            conversation_id,
            extension_tools: vec![],
        };

        let dispatched = match bus
            .publish(
                PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                    .with_conversation_id(conversation_id)
                    .with_interface(format!("{:?}", Interface::Scheduler))
                    .with_user_id("scheduler"),
            )
            .await
        {
            Ok(_) => {
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

/// Compute the next occurrence after now for a cron expression.
/// Accepts both 5-field (standard) and 7-field (with seconds) expressions.
fn compute_next_run(cron_expr: &str) -> Option<chrono::DateTime<Utc>> {
    let schedule = Schedule::from_str(cron_expr)
        .or_else(|_| Schedule::from_str(&format!("0 {}", cron_expr)))
        .ok()?;
    schedule.upcoming(Utc).next()
}

/// Read `HEARTBEAT.md` (from the configured path) and dispatch its contents
/// as a [`TurnRequest`] through the message bus.
///
/// Does nothing (silently) if the file does not exist or is empty.
async fn run_heartbeat(orchestrator: &Orchestrator) -> Result<()> {
    let heartbeat_path = orchestrator.heartbeat_path();

    if !heartbeat_path.exists() {
        return Ok(());
    }

    let raw = std::fs::read_to_string(heartbeat_path)?;
    let prompt = strip_html_comments(&raw);

    if prompt.is_empty() {
        return Ok(());
    }

    info!("Dispatching heartbeat from {}", heartbeat_path.display());

    let conversation_id = Uuid::new_v4();
    let turn_req = bus_messages::TurnRequest {
        prompt,
        conversation_id,
        extension_tools: vec![],
    };

    orchestrator
        .bus()
        .publish(
            PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                .with_conversation_id(conversation_id)
                .with_interface(format!("{:?}", Interface::Scheduler))
                .with_user_id("heartbeat"),
        )
        .await?;

    info!(
        conversation_id = %conversation_id,
        "Heartbeat dispatched to bus"
    );

    Ok(())
}
