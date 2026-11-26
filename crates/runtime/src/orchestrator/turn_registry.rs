//! One registry of the turns in flight on this orchestrator.
//!
//! A turn holds a [`TurnLease`] for as long as it runs. Leases are either
//! *shared* (an ordinary turn; several may coexist, as messengers and the
//! scheduler can overlap) or *exclusive* (an edit or regenerate that moves the
//! conversation's active leaf and must not race an append).
//!
//! The registry lives in this process only. Several services may share one
//! database and the bus may be NATS, so it cannot see turns another process
//! runs.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use uuid::Uuid;

/// How a turn shares its conversation with other turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnMode {
    /// An ordinary turn. Refused while an exclusive lease is held.
    Shared,
    /// A branching turn. Refused while any lease is held.
    Exclusive,
}

#[derive(Default)]
struct ConversationTurns {
    request_ids: HashSet<Uuid>,
    exclusive: bool,
}

type Turns = Arc<Mutex<HashMap<Uuid, ConversationTurns>>>;

/// Cheap, cloneable handle to the process's turn registry.
#[derive(Clone, Default)]
pub struct TurnRegistry {
    turns: Turns,
}

/// Proof that a turn is registered. Releases the turn on `Drop`, so every exit
/// path (success, error, cancel, a dropped future) clears it.
#[must_use = "a lease releases its turn as soon as it is dropped"]
pub struct TurnLease {
    turns: Turns,
    conversation_id: Uuid,
    request_id: Uuid,
}

impl std::fmt::Debug for TurnLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TurnLease")
            .field("conversation_id", &self.conversation_id)
            .field("request_id", &self.request_id)
            .finish()
    }
}

fn lock(turns: &Turns) -> MutexGuard<'_, HashMap<Uuid, ConversationTurns>> {
    // The guarded maps stay consistent across a panic, so a poisoned lock is
    // still safe to use.
    turns.lock().unwrap_or_else(PoisonError::into_inner)
}

impl TurnRegistry {
    /// Register `request_id` as running in `conversation_id`, or `None` when
    /// the conversation's current leases forbid it (or `request_id` is
    /// already registered there). The check and the insert are one atomic
    /// step.
    pub fn try_begin(
        &self,
        conversation_id: Uuid,
        request_id: Uuid,
        mode: TurnMode,
    ) -> Option<TurnLease> {
        let mut turns = lock(&self.turns);
        let entry = turns.entry(conversation_id).or_default();
        let admitted = match mode {
            TurnMode::Shared => !entry.exclusive,
            TurnMode::Exclusive => entry.request_ids.is_empty(),
        };
        if !admitted || !entry.request_ids.insert(request_id) {
            if entry.request_ids.is_empty() {
                turns.remove(&conversation_id);
            }
            return None;
        }
        entry.exclusive = mode == TurnMode::Exclusive;
        Some(TurnLease {
            turns: self.turns.clone(),
            conversation_id,
            request_id,
        })
    }

    /// True while any turn holds a lease on `conversation_id`.
    pub fn in_flight(&self, conversation_id: Uuid) -> bool {
        lock(&self.turns).contains_key(&conversation_id)
    }

    /// Request ids of the turns running in `conversation_id`.
    pub fn request_ids(&self, conversation_id: Uuid) -> Vec<Uuid> {
        lock(&self.turns)
            .get(&conversation_id)
            .map(|entry| entry.request_ids.iter().copied().collect())
            .unwrap_or_default()
    }

    /// True when `request_id` holds a lease in any conversation.
    pub fn is_registered(&self, request_id: Uuid) -> bool {
        lock(&self.turns)
            .values()
            .any(|entry| entry.request_ids.contains(&request_id))
    }
}

impl TurnLease {
    /// The conversation this lease covers.
    pub fn conversation_id(&self) -> Uuid {
        self.conversation_id
    }

    /// The request id of the leased turn.
    pub fn request_id(&self) -> Uuid {
        self.request_id
    }
}

impl Drop for TurnLease {
    fn drop(&mut self) {
        let mut turns = lock(&self.turns);
        if let Some(entry) = turns.get_mut(&self.conversation_id) {
            entry.request_ids.remove(&self.request_id);
            if entry.request_ids.is_empty() {
                turns.remove(&self.conversation_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (Uuid, Uuid) {
        (Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn concurrent_exclusive_begins_admit_exactly_one() {
        let registry = TurnRegistry::default();
        let conv = Uuid::new_v4();
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let registry = registry.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    registry.try_begin(conv, Uuid::new_v4(), TurnMode::Exclusive)
                })
            })
            .collect();
        let leases: Vec<_> = handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect();
        assert_eq!(leases.len(), 1, "exactly one exclusive lease is admitted");
    }

    #[test]
    fn shared_leases_coexist() {
        let registry = TurnRegistry::default();
        let conv = Uuid::new_v4();
        let a = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
        let b = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
        assert!(a.is_some() && b.is_some(), "shared leases may overlap");
    }

    #[test]
    fn shared_is_refused_while_exclusive_is_held() {
        let registry = TurnRegistry::default();
        let (conv, other) = ids();
        let _exclusive = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Exclusive);
        assert!(registry.try_begin(conv, other, TurnMode::Shared).is_none());
    }

    #[test]
    fn exclusive_is_refused_while_shared_is_held() {
        let registry = TurnRegistry::default();
        let conv = Uuid::new_v4();
        let _shared = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
        assert!(
            registry
                .try_begin(conv, Uuid::new_v4(), TurnMode::Exclusive)
                .is_none()
        );
    }

    #[test]
    fn other_conversations_are_independent() {
        let registry = TurnRegistry::default();
        let _held = registry.try_begin(Uuid::new_v4(), Uuid::new_v4(), TurnMode::Exclusive);
        assert!(
            registry
                .try_begin(Uuid::new_v4(), Uuid::new_v4(), TurnMode::Exclusive)
                .is_some()
        );
    }

    #[test]
    fn a_request_id_registers_only_once_per_conversation() {
        let registry = TurnRegistry::default();
        let (conv, request) = ids();
        let _first = registry.try_begin(conv, request, TurnMode::Shared);
        assert!(
            registry
                .try_begin(conv, request, TurnMode::Shared)
                .is_none()
        );
    }

    #[test]
    fn dropping_a_lease_releases_it() {
        let registry = TurnRegistry::default();
        let (conv, request) = ids();
        let lease = registry.try_begin(conv, request, TurnMode::Exclusive);
        assert!(lease.is_some());
        drop(lease);
        assert!(!registry.in_flight(conv));
        assert!(!registry.is_registered(request));
        assert!(
            registry
                .try_begin(conv, Uuid::new_v4(), TurnMode::Exclusive)
                .is_some(),
            "the conversation is free again"
        );
    }

    #[tokio::test]
    async fn a_dropped_future_releases_its_lease() {
        let registry = TurnRegistry::default();
        let conv = Uuid::new_v4();
        let lease = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
        let task = tokio::spawn(async move {
            let _lease = lease;
            std::future::pending::<()>().await;
        });
        tokio::task::yield_now().await;
        assert!(registry.in_flight(conv));
        task.abort();
        let _ = task.await;
        assert!(!registry.in_flight(conv), "abort drops the future's lease");
    }

    #[test]
    fn an_error_path_releases_its_lease() {
        fn fails(registry: &TurnRegistry, conv: Uuid) -> anyhow::Result<()> {
            let _lease = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
            anyhow::bail!("turn failed");
        }
        let registry = TurnRegistry::default();
        let conv = Uuid::new_v4();
        assert!(fails(&registry, conv).is_err());
        assert!(!registry.in_flight(conv));
    }

    #[test]
    fn in_flight_is_true_exactly_while_a_lease_is_held() {
        let registry = TurnRegistry::default();
        let (conv, request) = ids();
        assert!(!registry.in_flight(conv));
        let lease = registry.try_begin(conv, request, TurnMode::Shared);
        assert!(registry.in_flight(conv));
        assert_eq!(registry.request_ids(conv), vec![request]);
        assert!(registry.is_registered(request));
        drop(lease);
        assert!(!registry.in_flight(conv));
        assert!(registry.request_ids(conv).is_empty());
    }

    #[test]
    fn releasing_one_shared_lease_keeps_the_others() {
        let registry = TurnRegistry::default();
        let conv = Uuid::new_v4();
        let first = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
        let _second = registry.try_begin(conv, Uuid::new_v4(), TurnMode::Shared);
        drop(first);
        assert!(registry.in_flight(conv));
        assert!(
            registry
                .try_begin(conv, Uuid::new_v4(), TurnMode::Exclusive)
                .is_none()
        );
    }
}
