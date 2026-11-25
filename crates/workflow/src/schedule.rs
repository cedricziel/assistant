//! Schedule trigger adapter: queues runs for cron-triggered workflows.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use assistant_storage::{StorageLayer, WorkflowNodeKind, WorkflowStore, WorkflowTriggerKind};
use chrono::{DateTime, Utc};
use cron::Schedule;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Spawn schedule trigger adapter that creates runs for due schedule nodes.
pub fn spawn_schedule_trigger_adapter(
    storage: Arc<StorageLayer>,
    poll_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!(?poll_interval, "Workflow schedule adapter started");
        loop {
            tokio::time::sleep(poll_interval).await;
            if let Err(err) = process_schedule_triggers(&storage).await {
                error!(error = %err, "Workflow schedule adapter iteration failed");
            }
        }
    })
}

async fn process_schedule_triggers(storage: &StorageLayer) -> Result<()> {
    let store = storage.workflow_store();
    let workflows = store.list().await?;
    let now = SystemClock.now();

    for workflow in workflows.into_iter().filter(|w| w.active) {
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
            if trigger_type != "schedule" {
                continue;
            }

            let cron_expr = node
                .config
                .get("cron")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if cron_expr.is_empty() {
                warn!(workflow_id = %workflow.id, node_id = %node.id, "schedule trigger missing cron");
                continue;
            }

            if !schedule_trigger_due(&store, workflow.id, node.id.as_str(), cron_expr, now).await? {
                continue;
            }

            let payload = serde_json::json!({
                "trigger_node_id": node.id,
                "cron": cron_expr,
                "scheduled_at": now.to_rfc3339(),
            });
            let run_id = store
                .create_run(workflow.id, WorkflowTriggerKind::Schedule, &payload)
                .await?;
            info!(workflow_id = %workflow.id, node_id = %node.id, run_id = %run_id, "Scheduled trigger queued workflow run");
        }
    }

    Ok(())
}

async fn schedule_trigger_due(
    store: &dyn WorkflowStore,
    workflow_id: Uuid,
    trigger_node_id: &str,
    cron_expr: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    let runs = store.list_runs(workflow_id, 200).await?;
    let latest_for_node = runs.into_iter().find(|run| {
        run.trigger_type == "schedule"
            && run
                .trigger_payload
                .get("trigger_node_id")
                .and_then(|v| v.as_str())
                == Some(trigger_node_id)
    });

    let Some(schedule) = parse_schedule(cron_expr) else {
        warn!(workflow_id = %workflow_id, trigger_node_id, cron_expr, "Invalid schedule cron expression");
        return Ok(false);
    };

    if let Some(last_run) = latest_for_node {
        let next = schedule.after(&last_run.started_at).next();
        return Ok(next.map(|t| t <= now).unwrap_or(false));
    }

    Ok(true)
}

fn parse_schedule(expr: &str) -> Option<Schedule> {
    Schedule::from_str(expr)
        .or_else(|_| Schedule::from_str(&format!("0 {expr}")))
        .ok()
}
