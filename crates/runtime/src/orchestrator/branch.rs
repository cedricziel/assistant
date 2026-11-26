//! Branching while a turn waits for the user (ADR-0018, D7a).
//!
//! Edit, regenerate and switch-branch need the conversation to themselves. A
//! turn that is blocked on a prompt (a tool approval, a question or a surface
//! wait) cannot make progress, and the only way to unblock it is to answer a
//! prompt on a branch the user is leaving. So such a turn is cancelled. A turn
//! that is still working is never cancelled here: the user can press Stop.

use std::collections::HashSet;
use std::time::Duration;

use tracing::{info, warn};
use uuid::Uuid;

use assistant_storage::InteractionStatus;

use super::{Orchestrator, TurnLease, TurnMode};

/// How long a branch operation waits for the runs it cancelled to release
/// their leases. The same ceiling the client's "Stopping…" state uses.
pub const BRANCH_CANCEL_WAIT: Duration = Duration::from_secs(10);

/// How often the wait checks whether the cancelled runs let go.
const RELEASE_POLL: Duration = Duration::from_millis(25);

/// The conversation is busy and the branch operation must not proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchConflict;

impl std::fmt::Display for BranchConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a turn is still running in this conversation")
    }
}

impl std::error::Error for BranchConflict {}

impl Orchestrator {
    /// Acquire the exclusive lease a branch operation needs on
    /// `conversation_id`, as `request_id`.
    ///
    /// When the lease is free it is granted at once. When turns hold the
    /// conversation, and **every** one of them waits on a pending interaction,
    /// those turns are cancelled through [`Self::cancel_turn`] (the path
    /// `/stop` uses), their interactions resolve as `cancelled`, and the lease
    /// is granted once they let go, within [`BRANCH_CANCEL_WAIT`].
    ///
    /// # Errors
    ///
    /// [`BranchConflict`] when any turn is still working (nothing is
    /// cancelled), when the interactions cannot be read, or when the cancelled
    /// turns do not release the conversation in time.
    pub async fn acquire_exclusive_for_branch(
        &self,
        conversation_id: Uuid,
        request_id: Uuid,
    ) -> Result<TurnLease, BranchConflict> {
        if let Some(lease) = self.try_exclusive(conversation_id, request_id) {
            return Ok(lease);
        }
        let held = self.turn_registry.request_ids(conversation_id);
        if held.is_empty() {
            // The holder released between the two calls.
            return self
                .try_exclusive(conversation_id, request_id)
                .ok_or(BranchConflict);
        }
        let waiting = self.runs_waiting_for_user(conversation_id).await?;
        if !held.iter().all(|run| waiting.contains(&run.to_string())) {
            return Err(BranchConflict);
        }

        info!(%conversation_id, runs = held.len(), "branch: cancelling turns waiting for the user");
        for run in &held {
            self.cancel_turn(*run).await;
        }
        // Resolve after the cancel so the turn cannot continue on a denied
        // answer. The end-of-run cleanup does the same, and is idempotent.
        for run in &held {
            if let Err(e) = self
                .interactions
                .cancel_pending_for_run(&run.to_string())
                .await
            {
                warn!(run_id = %run, "branch: could not cancel pending interactions: {e:#}");
            }
        }
        self.wait_for_exclusive(conversation_id, request_id).await
    }

    fn try_exclusive(&self, conversation_id: Uuid, request_id: Uuid) -> Option<TurnLease> {
        self.turn_registry
            .try_begin(conversation_id, request_id, TurnMode::Exclusive)
    }

    /// Run ids of `conversation_id` that have a pending interaction.
    async fn runs_waiting_for_user(
        &self,
        conversation_id: Uuid,
    ) -> Result<HashSet<String>, BranchConflict> {
        self.interactions
            .list_for_conversation(
                &conversation_id.to_string(),
                Some(InteractionStatus::Pending),
            )
            .await
            .map(|rows| rows.into_iter().map(|row| row.run_id).collect())
            .map_err(|e| {
                warn!(%conversation_id, "branch: could not read pending interactions: {e:#}");
                BranchConflict
            })
    }

    async fn wait_for_exclusive(
        &self,
        conversation_id: Uuid,
        request_id: Uuid,
    ) -> Result<TurnLease, BranchConflict> {
        let deadline = tokio::time::Instant::now() + BRANCH_CANCEL_WAIT;
        loop {
            if let Some(lease) = self.try_exclusive(conversation_id, request_id) {
                return Ok(lease);
            }
            if tokio::time::Instant::now() >= deadline {
                warn!(%conversation_id, "branch: cancelled turns did not release in time");
                return Err(BranchConflict);
            }
            tokio::time::sleep(RELEASE_POLL).await;
        }
    }
}
