//! Background scheduler — polls for due scheduled tasks and runs them via the
//! ReAct orchestrator.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::Interface;
use assistant_storage::StorageLayer;
use chrono::Utc;
use cron::Schedule;
use tracing::{error, info};
use uuid::Uuid;

use crate::orchestrator::ReactOrchestrator;

/// Spawn a background tokio task that checks for due scheduled tasks every
/// `poll_interval` and runs them through the orchestrator.
pub fn spawn_scheduler(
    storage: Arc<StorageLayer>,
    orchestrator: Arc<ReactOrchestrator>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Scheduler started (poll interval: {:?})", poll_interval);
        loop {
            tokio::time::sleep(poll_interval).await;

            if let Err(e) = run_due_tasks(&storage, &orchestrator).await {
                error!("Scheduler error: {e}");
            }
        }
    })
}

async fn run_due_tasks(storage: &StorageLayer, orchestrator: &ReactOrchestrator) -> Result<()> {
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
