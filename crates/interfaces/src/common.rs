//! Shared helpers used across multiple interface adapters.

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

/// Simple LCG-based float in `[0, 1)` without pulling in `rand`.
pub fn rand_jitter() -> f64 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let v = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (v >> 32) as f64 / ((1u64 << 32) as f64)
}
