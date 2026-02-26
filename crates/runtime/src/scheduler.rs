//! Background scheduler — polls for due scheduled tasks and runs them via the
//! orchestrator. Also drives the heartbeat loop (reads `HEARTBEAT.md` from the
//! configured memory path).

use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use assistant_core::{strip_html_comments, Interface};
use assistant_storage::StorageLayer;
use chrono::Utc;
use cron::Schedule;
use opentelemetry::trace::{Span, TraceContextExt, Tracer};
use opentelemetry::{global, Context as OtelContext, KeyValue};
use tracing::{error, info};
use uuid::Uuid;

use crate::orchestrator::Orchestrator;

/// How often the heartbeat prompt is run (30 minutes).
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// Spawn a background tokio task that:
/// 1. Checks for due scheduled tasks every `poll_interval`.
/// 2. Reads `HEARTBEAT.md` (from the configured memory path) as a prompt
///    through the orchestrator every 30 minutes (if the file exists and is
///    non-empty).
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
                if let Err(e) = run_heartbeat(&orchestrator).await {
                    error!("Heartbeat error: {e}");
                }
                last_heartbeat = Instant::now();
            }
        }
    })
}

async fn run_due_tasks(storage: &StorageLayer, orchestrator: &Orchestrator) -> Result<()> {
    let now = Utc::now();
    let task_store = storage.scheduled_task_store();
    let due = task_store.due_tasks(now).await?;

    for task in due {
        info!(task_name = %task.name, "Running scheduled task");

        let conversation_id = Uuid::new_v4();
        let result = orchestrator
            .run_turn(&task.prompt, conversation_id, Interface::Cli, None)
            .await;

        match result {
            Ok(turn) => {
                info!(
                    task_name = %task.name,
                    answer_len = turn.answer.len(),
                    "Scheduled task completed"
                );
            }
            Err(e) => {
                error!(task_name = %task.name, error = %e, "Scheduled task failed");
            }
        }

        if task.once {
            // One-shot task: record the run and disable it.
            task_store.record_run(task.id, now, None).await?;
            task_store.disable(task.id).await?;
            info!(task_name = %task.name, "One-shot task disabled after execution");
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

/// Read `HEARTBEAT.md` (from the configured path) and run its contents through
/// the orchestrator.
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

    info!("Running heartbeat from {}", heartbeat_path.display());

    // Start a root span so heartbeat traces are easily identifiable.
    let tracer = global::tracer("sysiphos.heartbeat");
    let conversation_id = Uuid::new_v4();
    let mut span = tracer.start("heartbeat");
    span.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    let heartbeat_cx = OtelContext::current().with_span(span);

    match orchestrator
        .run_turn(
            &prompt,
            conversation_id,
            Interface::Cli,
            Some(&heartbeat_cx),
        )
        .await
    {
        Ok(turn) => {
            info!(answer_len = turn.answer.len(), "Heartbeat completed");
        }
        Err(e) => {
            error!(error = %e, "Heartbeat failed");
        }
    }

    Ok(())
}
