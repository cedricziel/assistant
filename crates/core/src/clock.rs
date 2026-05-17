//! Workspace clock abstraction.
//!
//! Provides the [`Clock`] trait abstracting wall-clock and monotonic time
//! access, the production [`SystemClock`] implementation, and the test-only
//! [`FakeClock`] implementation. The trait + impls live in `assistant-core`
//! alongside other shared workspace primitives.
//!
//! ## Adoption
//!
//! Production code that needs the current time SHALL accept
//! `Arc<dyn Clock>` at construction and default to
//! `Arc::new(SystemClock)` via a builder. Direct calls to
//! [`chrono::Utc::now`] / [`std::time::SystemTime::now`] / [`std::time::Instant::now`]
//! are forbidden in non-test production code; the workspace lint
//! `tests/workspace_clock_lint.rs` (added in a later phase of
//! `workspace-test-coverage-floor`) will enforce this once every crate has
//! migrated.
//!
//! Tests use [`FakeClock`] to seed and advance virtual time:
//!
//! ```
//! use assistant_core::clock::{Clock, FakeClock};
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! let fake = Arc::new(FakeClock::new_at_seed());
//! let t0 = fake.now();
//! fake.advance(Duration::from_secs(60));
//! let t1 = fake.now();
//! assert_eq!((t1 - t0).num_seconds(), 60);
//! ```

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};

/// Abstract access to wall-clock and monotonic time.
///
/// Production: `SystemClock` (wraps `chrono::Utc::now` and `std::time::Instant::now`).
/// Tests: `FakeClock` (seedable + advanceable virtual time).
pub trait Clock: Send + Sync {
    /// The current wall-clock time as a `DateTime<Utc>`.
    fn now(&self) -> DateTime<Utc>;

    /// A monotonic instant. For `SystemClock`, the underlying
    /// `std::time::Instant`. For `FakeClock`, derived from the seeded
    /// wall-clock time so that `now_instant` advances in lockstep with
    /// `now`.
    fn now_instant(&self) -> Instant;
}

/// Production [`Clock`] implementation. Wraps `chrono::Utc::now` and
/// `std::time::Instant::now`.
///
/// `SystemClock` is the only call site of the underlying system clocks
/// permitted by the workspace lint policy; every other production site
/// MUST route through `Arc<dyn Clock>`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn now_instant(&self) -> Instant {
        Instant::now()
    }
}

/// Test-only [`Clock`] implementation backed by a seeded `DateTime<Utc>`.
///
/// `FakeClock` is interior-mutable: cloned handles share the same
/// underlying time, so a test can hold an `Arc<dyn Clock>` referring to
/// a `FakeClock` *and* hold the concrete `FakeClock` to advance it.
///
/// Despite its name and intended use, `FakeClock` is plain `pub` (not
/// `#[cfg(test)]`-gated) — the workspace's testability policy mandates
/// that in-memory / fake implementations live next to their production
/// peers and are usable from any crate's tests via the test-support
/// prelude. Production code that constructs `FakeClock` is flagged by
/// `tests/workspace_test_impls_in_prod.rs`.
#[derive(Debug, Clone)]
pub struct FakeClock {
    inner: Arc<FakeClockInner>,
}

#[derive(Debug)]
struct FakeClockInner {
    state: Mutex<FakeClockState>,
}

#[derive(Debug)]
struct FakeClockState {
    wall: DateTime<Utc>,
    /// Origin `Instant` recorded at construction. `now_instant()` returns
    /// `origin + (wall - wall_origin)`.
    origin_instant: Instant,
    wall_origin: DateTime<Utc>,
}

impl FakeClock {
    /// Construct a [`FakeClock`] seeded at the given wall-clock time.
    pub fn new(seed: DateTime<Utc>) -> Self {
        let origin_instant = Instant::now();
        Self {
            inner: Arc::new(FakeClockInner {
                state: Mutex::new(FakeClockState {
                    wall: seed,
                    origin_instant,
                    wall_origin: seed,
                }),
            }),
        }
    }

    /// Construct a [`FakeClock`] seeded at a deterministic, well-known
    /// epoch (`2026-01-01T00:00:00Z`). Convenient for test fixtures.
    pub fn new_at_seed() -> Self {
        let seed = Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .unwrap_or_else(|| {
                // Fallback chain in the extremely unlikely event chrono
                // rejects the seed literal — keep this code panic-free.
                Utc.timestamp_opt(0, 0)
                    .single()
                    .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default())
            });
        Self::new(seed)
    }

    /// Advance virtual time by the given duration.
    pub fn advance(&self, by: Duration) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.wall += chrono::Duration::from_std(by).unwrap_or(chrono::Duration::zero());
        }
    }

    /// Set virtual time to a specific wall-clock instant.
    pub fn set(&self, to: DateTime<Utc>) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.wall = to;
            // Re-anchor the Instant origin so monotonic time stays in
            // sync with the new wall-clock value.
            state.origin_instant = Instant::now();
            state.wall_origin = to;
        }
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new_at_seed()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        match self.inner.state.lock() {
            Ok(state) => state.wall,
            Err(poisoned) => poisoned.into_inner().wall,
        }
    }

    fn now_instant(&self) -> Instant {
        match self.inner.state.lock() {
            Ok(state) => {
                let delta_chrono = state.wall - state.wall_origin;
                let delta = delta_chrono
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(0));
                state.origin_instant + delta
            }
            Err(poisoned) => {
                let state = poisoned.into_inner();
                let delta_chrono = state.wall - state.wall_origin;
                let delta = delta_chrono
                    .to_std()
                    .unwrap_or(std::time::Duration::from_secs(0));
                state.origin_instant + delta
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 17, 8, 30, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn system_clock_returns_recent_wall_clock_time() {
        let clock = SystemClock;
        let before = Utc::now();
        let now = clock.now();
        let after = Utc::now();
        assert!(
            now >= before && now <= after,
            "SystemClock::now() must return a time within [before, after]"
        );
    }

    #[test]
    fn fake_clock_returns_seeded_time() {
        let clock = FakeClock::new(seed_time());
        assert_eq!(clock.now(), seed_time(), "FakeClock must return the seed");
    }

    #[test]
    fn fake_clock_advance_moves_wall_clock() {
        let clock = FakeClock::new(seed_time());
        clock.advance(Duration::from_secs(90));
        let expected = seed_time() + chrono::Duration::seconds(90);
        assert_eq!(
            clock.now(),
            expected,
            "advance(90s) must move wall clock by 90s"
        );
    }

    #[test]
    fn fake_clock_set_overrides_time() {
        let clock = FakeClock::new(seed_time());
        let new_time = seed_time() + chrono::Duration::hours(2);
        clock.set(new_time);
        assert_eq!(clock.now(), new_time, "set(x) must replace the wall clock");
    }

    #[test]
    fn fake_clock_instant_advances_with_wall_clock() {
        let clock = FakeClock::new(seed_time());
        let t0 = clock.now_instant();
        clock.advance(Duration::from_secs(30));
        let t1 = clock.now_instant();
        assert!(
            t1 >= t0 + Duration::from_secs(29),
            "now_instant must advance with the wall clock"
        );
    }

    #[test]
    fn fake_clock_is_clonable_and_shared() {
        let a = FakeClock::new(seed_time());
        let b = a.clone();
        a.advance(Duration::from_secs(45));
        assert_eq!(
            b.now(),
            seed_time() + chrono::Duration::seconds(45),
            "cloned FakeClock handles must share state"
        );
    }

    #[test]
    fn clock_trait_is_object_safe() {
        // Compile-time assertion that Clock is dyn-compatible.
        let _: Arc<dyn Clock> = Arc::new(SystemClock);
        let _: Arc<dyn Clock> = Arc::new(FakeClock::new_at_seed());
    }

    #[test]
    fn fake_clock_default_uses_well_known_seed() {
        let a = FakeClock::default();
        let b = FakeClock::new_at_seed();
        assert_eq!(
            a.now(),
            b.now(),
            "Default::default() must equal new_at_seed()"
        );
    }
}
