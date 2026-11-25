//! Replay protection for Nextcloud Talk webhooks.
//!
//! The Talk bot protocol signs `random + body` but carries no timestamp, so a
//! captured request would verify forever. [`ReplayGuard`] remembers each
//! accepted `X-Nextcloud-Talk-Random` value for a bounded window and rejects
//! reuse. Memory is capped: once full, the oldest entry is forgotten.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use assistant_core::clock::Clock;

/// How long an accepted `random` value is remembered.
pub(super) const REPLAY_WINDOW: Duration = Duration::from_secs(10 * 60);

/// Maximum number of remembered `random` values.
pub(super) const REPLAY_CAPACITY: usize = 10_000;

#[derive(Default)]
struct Seen {
    at: HashMap<String, Instant>,
    order: VecDeque<String>,
}

pub(super) struct ReplayGuard {
    clock: Arc<dyn Clock>,
    ttl: Duration,
    capacity: usize,
    seen: Mutex<Seen>,
}

impl ReplayGuard {
    pub(super) fn new(clock: Arc<dyn Clock>, ttl: Duration, capacity: usize) -> Self {
        Self {
            clock,
            ttl,
            capacity,
            seen: Mutex::new(Seen::default()),
        }
    }

    /// Record `random` and return `true` if it has not been seen inside the
    /// window; return `false` for a replay.
    pub(super) fn check_and_record(&self, random: &str) -> bool {
        let now = self.clock.now_instant();
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());

        while let Some(oldest) = seen.order.front() {
            let expired = seen
                .at
                .get(oldest)
                .is_none_or(|at| now.duration_since(*at) >= self.ttl);
            if !expired && seen.order.len() < self.capacity {
                break;
            }
            if let Some(key) = seen.order.pop_front() {
                seen.at.remove(&key);
            }
        }

        if seen.at.contains_key(random) {
            return false;
        }
        seen.at.insert(random.to_owned(), now);
        seen.order.push_back(random.to_owned());
        true
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.seen.lock().unwrap_or_else(|e| e.into_inner()).at.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use assistant_core::clock::FakeClock;

    use super::*;

    fn guard(clock: &Arc<FakeClock>, ttl_secs: u64, capacity: usize) -> ReplayGuard {
        ReplayGuard::new(clock.clone(), Duration::from_secs(ttl_secs), capacity)
    }

    #[test]
    fn first_sighting_is_fresh_second_is_replay() {
        let clock = Arc::new(FakeClock::new_at_seed());
        let g = guard(&clock, 60, 16);
        assert!(g.check_and_record("abc"), "first use must be accepted");
        assert!(!g.check_and_record("abc"), "reuse must be rejected");
        assert!(g.check_and_record("def"), "a different value is fresh");
    }

    #[test]
    fn value_is_accepted_again_after_the_window() {
        let clock = Arc::new(FakeClock::new_at_seed());
        let g = guard(&clock, 60, 16);
        assert!(g.check_and_record("abc"));
        clock.advance(Duration::from_secs(61));
        assert!(
            g.check_and_record("abc"),
            "entries older than the window are forgotten"
        );
    }

    #[test]
    fn replay_inside_the_window_is_rejected() {
        let clock = Arc::new(FakeClock::new_at_seed());
        let g = guard(&clock, 60, 16);
        assert!(g.check_and_record("abc"));
        clock.advance(Duration::from_secs(59));
        assert!(!g.check_and_record("abc"));
    }

    #[test]
    fn capacity_bounds_memory_by_evicting_the_oldest() {
        let clock = Arc::new(FakeClock::new_at_seed());
        let g = guard(&clock, 3600, 3);
        for (i, r) in ["a", "b", "c", "d"].iter().enumerate() {
            assert!(g.check_and_record(r));
            clock.advance(Duration::from_secs(i as u64 + 1));
        }
        assert_eq!(g.len(), 3, "never exceeds capacity");
        assert!(g.check_and_record("a"), "oldest entry was evicted");
        assert!(!g.check_and_record("d"), "newest entry is retained");
    }
}
