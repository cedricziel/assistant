//! Crash-recovery sweep: reclaim stale bus messages and close orphaned
//! SSE event streams left over after a server restart.

use std::time::Duration;

use tracing::{info, warn};

use assistant_core::clock::{Clock as _, SystemClock};
use assistant_core::{MessageBus, topic};
use assistant_storage::{
    ConversationEventStore, InteractionStatus, InteractionStore, StorageLayer,
};

use crate::interaction::{WAITER_STALE_AFTER, resolved_event};
use crate::projection::{SseProjector, StreamProjector as _};

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
///    terminal event and can retry cleanly. Each interaction of the run whose
///    resolution is not yet in the log gets an `interaction_resolved` event
///    first.
///
/// Before step 2, `pending` interactions whose waiter went away are expired.
/// That cannot wait for the orphan path: it skips runs with an active bus
/// message, and NATS always reports one.
pub(crate) async fn reap_stale_and_recover(storage: &StorageLayer, bus: &dyn MessageBus) {
    // Step 1: reclaim stale bus messages.
    match bus.reap_stale(STALE_CLAIM_TIMEOUT).await {
        Ok(n) if n > 0 => info!("Reaped {n} stale bus message(s)"),
        Ok(_) => {}
        Err(e) => warn!("Failed to reap stale bus messages: {e}"),
    }

    let interactions = storage.interaction_store();
    let dead_since =
        SystemClock.now() - chrono::Duration::from_std(WAITER_STALE_AFTER).unwrap_or_default();
    match interactions.expire_dead(dead_since).await {
        Ok(ids) if !ids.is_empty() => info!(count = ids.len(), "Expired dead interaction(s)"),
        Ok(_) => {}
        Err(e) => warn!("Failed to expire dead interactions: {e}"),
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

        if let Err(e) =
            log_interaction_resolutions(&event_store, &interactions, &run_id, &conversation_id)
                .await
        {
            warn!(
                run_id = %run_id,
                error = %e,
                "Failed to log interaction resolutions for orphaned run"
            );
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

/// Append an `interaction_resolved` event for every interaction of the run that
/// has none in the log, and mark each one emitted. A row still `pending` has no
/// run left to answer it, so it expires first.
async fn log_interaction_resolutions(
    event_store: &impl ConversationEventStore,
    interactions: &impl InteractionStore,
    run_id: &str,
    conversation_id: &str,
) -> anyhow::Result<()> {
    let mut seq = event_store
        .list_events_since(run_id, 0)
        .await?
        .last()
        .map_or(0, |last| last.sequence + 1);
    for row in interactions.list_unemitted_for_run(run_id).await? {
        let row = if row.status == InteractionStatus::Pending {
            interactions
                .resolve_if_pending(&row.id, InteractionStatus::Expired, None, None)
                .await?;
            match interactions.get(&row.id).await? {
                Some(settled) => settled,
                None => continue,
            }
        } else {
            row
        };
        let Some(event) = resolved_event(&row) else {
            continue;
        };
        for frame in SseProjector.project(&event) {
            event_store
                .append_event(
                    run_id,
                    conversation_id,
                    seq,
                    &frame.event,
                    &frame.log_payload,
                )
                .await?;
            seq += 1;
        }
        interactions.mark_emitted(&row.id).await?;
    }
    Ok(())
}
