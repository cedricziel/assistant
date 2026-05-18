//! Crash-recovery sweep: reclaim stale bus messages and close orphaned
//! SSE event streams left over after a server restart.

use assistant_storage::ConversationEventStore as _;
use std::time::Duration;

use tracing::{info, warn};

use assistant_core::{MessageBus, topic};
use assistant_storage::StorageLayer;

/// How often stale bus messages and orphaned SSE runs are checked (5 minutes).
pub(super) const REAP_STALE_INTERVAL: Duration = Duration::from_secs(5 * 60);

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
        let has_active = match bus
            .has_active_message(&conversation_id, topic::TURN_REQUEST)
            .await
        {
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
