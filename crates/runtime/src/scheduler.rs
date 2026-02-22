//! Background scheduler — polls for due scheduled tasks and runs them via the
//! orchestrator. Also drives the heartbeat loop (`~/.assistant/HEARTBEAT.md`).

use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use assistant_core::Interface;
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
/// 2. Runs `~/.assistant/HEARTBEAT.md` as a prompt through the orchestrator every 30 minutes
///    (if the file exists and is non-empty).
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
            .run_turn(&task.prompt, conversation_id, Interface::Cli)
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

        // Compute the next run time from the cron expression.
        let next_run = compute_next_run(&task.cron_expr);
        task_store.record_run(task.id, now, next_run).await?;
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

/// Read `~/.assistant/HEARTBEAT.md` and run its contents through the orchestrator.
///
/// Does nothing (silently) if the file does not exist or is empty.
async fn run_heartbeat(orchestrator: &Orchestrator) -> Result<()> {
    let heartbeat_path = match dirs::home_dir() {
        Some(h) => h.join(".assistant").join("HEARTBEAT.md"),
        None => return Ok(()),
    };

    if !heartbeat_path.exists() {
        return Ok(());
    }

    let prompt = std::fs::read_to_string(&heartbeat_path)?;
    let prompt = prompt.trim().to_string();

    if prompt.is_empty() {
        return Ok(());
    }

    info!("Running heartbeat from {}", heartbeat_path.display());

    let conversation_id = Uuid::new_v4();
    match orchestrator
        .run_turn(&prompt, conversation_id, Interface::Cli)
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
