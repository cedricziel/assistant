//! Shared helpers used across multiple interface adapters.

use assistant_core::clock::{Clock, SystemClock};
use std::time::Duration;

/// Minimum reconnect delay.
pub const BACKOFF_MIN: Duration = Duration::from_secs(1);
/// Maximum reconnect delay.
pub const BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Sleep for the current backoff duration (with ±10 % jitter), then double it.
///
/// Cancels early if `stop_rx` fires.
pub async fn sleep_backoff(
    backoff: &mut Duration,
    stop_rx: &mut tokio::sync::watch::Receiver<bool>,
) {
    // Add ±10% jitter.
    let jitter_pct = (rand_jitter() - 0.5) * 0.2; // -10% to +10%
    let secs = backoff.as_secs_f64() * (1.0 + jitter_pct);
    let sleep_dur = Duration::from_secs_f64(secs.max(0.1));

    tokio::select! {
        _ = tokio::time::sleep(sleep_dur) => {}
        _ = stop_rx.changed() => {}
    }

    // Exponential increase, capped.
    *backoff = (*backoff * 2).min(BACKOFF_MAX);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rand_jitter_in_unit_interval() {
        for _ in 0..50 {
            let v = rand_jitter();
            assert!((0.0..1.0).contains(&v), "{v} not in [0,1)");
        }
    }

    #[tokio::test]
    async fn sleep_backoff_doubles_within_cap_when_stop_aborts() {
        // Use a tiny initial backoff + immediate stop so the test doesn't
        // actually sleep for any meaningful time. We only care about the
        // doubling side-effect.
        let mut backoff = Duration::from_secs(1);
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        tx.send(true).unwrap();
        sleep_backoff(&mut backoff, &mut rx).await;
        assert_eq!(backoff, Duration::from_secs(2));
    }

    #[tokio::test]
    async fn sleep_backoff_returns_early_when_stop_signal_fires() {
        let mut backoff = Duration::from_secs(60);
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        // Fire stop signal immediately; the select should pick the stop branch
        // and return without sleeping the full 60s.
        tx.send(true).unwrap();
        let start = std::time::Instant::now();
        sleep_backoff(&mut backoff, &mut rx).await;
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "expected early cancel"
        );
        // Backoff still doubles but is capped at BACKOFF_MAX.
        assert_eq!(backoff, BACKOFF_MAX);
    }

    #[tokio::test]
    async fn sleep_backoff_caps_at_max() {
        let mut backoff = BACKOFF_MAX;
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        tx.send(true).unwrap();
        sleep_backoff(&mut backoff, &mut rx).await;
        assert_eq!(backoff, BACKOFF_MAX, "must not exceed BACKOFF_MAX");
    }
}

/// Simple LCG-based float in `[0, 1)` without pulling in `rand`.
pub fn rand_jitter() -> f64 {
    // Use wall-clock subsec-nanos as the seed source. This is sufficient
    // randomness for jitter; security-sensitive randomness should use
    // `getrandom` instead.
    let seed = SystemClock.now().timestamp_subsec_nanos() as u64;
    let v = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (v >> 32) as f64 / ((1u64 << 32) as f64)
}
