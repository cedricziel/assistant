//! Cancellation registry for bus turns, keyed by request id.
//!
//! `submit_turn` registers an entry before publishing the request, the worker
//! marks it *claimed* when it picks the request up, and `cancel_turn` fires
//! the token. A cancel that lands before the claim leaves a bounded tombstone
//! so the worker can skip the request instead of running a turn nobody waits
//! for.

use std::collections::{HashMap, HashSet, VecDeque};

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Maximum number of cancelled-before-claim request ids remembered.
const TOMBSTONE_CAPACITY: usize = 256;

struct Entry {
    token: CancellationToken,
    claimed: bool,
}

/// What a worker should do with a request it has just taken off the bus.
pub(crate) enum Claim {
    /// The request was cancelled before any worker claimed it; skip it.
    Cancelled,
    /// The request is now claimed; run it under this token.
    Claimed(CancellationToken),
    /// No `submit_turn` waits on this request (e.g. scheduler turns); run it
    /// with a fresh, never-cancelled token.
    Untracked,
}

#[derive(Default)]
pub(crate) struct TurnCancellations {
    entries: HashMap<Uuid, Entry>,
    cancelled_order: VecDeque<Uuid>,
    cancelled: HashSet<Uuid>,
}

impl TurnCancellations {
    pub(crate) fn register(&mut self, request_id: Uuid, token: CancellationToken) {
        self.entries.insert(
            request_id,
            Entry {
                token,
                claimed: false,
            },
        );
    }

    pub(crate) fn contains(&self, request_id: Uuid) -> bool {
        self.entries.contains_key(&request_id)
    }

    /// Cancel the registered turn. Returns `false` when none is registered.
    pub(crate) fn cancel(&mut self, request_id: Uuid) -> bool {
        let Some(entry) = self.entries.get(&request_id) else {
            return false;
        };
        entry.token.cancel();
        if !entry.claimed {
            self.remember_cancelled(request_id);
        }
        true
    }

    pub(crate) fn claim(&mut self, request_id: Uuid) -> Claim {
        if self.cancelled.contains(&request_id) {
            return Claim::Cancelled;
        }
        match self.entries.get_mut(&request_id) {
            Some(entry) => {
                entry.claimed = true;
                Claim::Claimed(entry.token.clone())
            }
            None => Claim::Untracked,
        }
    }

    /// Deregister the turn. Returns whether a worker had claimed it, or
    /// `None` when it was not registered.
    pub(crate) fn remove(&mut self, request_id: Uuid) -> Option<bool> {
        self.entries.remove(&request_id).map(|entry| entry.claimed)
    }

    fn remember_cancelled(&mut self, request_id: Uuid) {
        if self.cancelled.insert(request_id) {
            self.cancelled_order.push_back(request_id);
            if self.cancelled_order.len() > TOMBSTONE_CAPACITY
                && let Some(oldest) = self.cancelled_order.pop_front()
            {
                self.cancelled.remove(&oldest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tombstones_are_bounded_and_evict_the_oldest() {
        let mut registry = TurnCancellations::default();
        let ids: Vec<Uuid> = (0..=TOMBSTONE_CAPACITY).map(|_| Uuid::new_v4()).collect();
        for id in &ids {
            registry.register(*id, CancellationToken::new());
            assert!(registry.cancel(*id));
        }

        assert!(
            matches!(registry.claim(ids[0]), Claim::Claimed(_)),
            "the 257th tombstone must evict the first"
        );
        assert!(
            matches!(registry.claim(ids[1]), Claim::Cancelled),
            "later tombstones must survive"
        );
    }
}
