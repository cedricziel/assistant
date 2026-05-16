//! Heartbeat dispatcher — reads `HEARTBEAT.md` and publishes its contents
//! as a `TurnRequest` on the bus every `HEARTBEAT_INTERVAL`.

use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use assistant_core::{strip_html_comments, types::conversation::Interface};

use super::tasks::dispatch_scheduler_turn_request;
use crate::orchestrator::Orchestrator;
use crate::otel_spans::start_interface_root_context;

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

    let storage = orchestrator.storage.clone();
    dispatch_scheduler_turn_request(
        &storage,
        orchestrator,
        prompt,
        conversation_id,
        &interface_cx,
    )
    .await?;

    info!(
        conversation_id = %conversation_id,
        "Heartbeat dispatched to bus"
    );

    Ok(())
}
