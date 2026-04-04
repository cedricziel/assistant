//! Common lifecycle trait for all chat interface runners.

use anyhow::Result;
use async_trait::async_trait;

/// Lifecycle contract for an assistant messaging interface.
///
/// All interface crates (Slack, Matrix, Mattermost, Nextcloud, Signal)
/// implement this trait.  It enables generic multi-interface launch code
/// and uniform graceful-shutdown wiring — for example, holding a
/// `Vec<Box<dyn InterfaceRunner>>` and joining them all concurrently.
///
/// # Example
///
/// ```rust,ignore
/// use assistant_runtime::InterfaceRunner;
/// use futures::future::join_all;
///
/// async fn run_all(runners: Vec<Box<dyn InterfaceRunner>>) {
///     join_all(runners.iter().map(|r| r.run())).await;
/// }
/// ```
#[async_trait]
pub trait InterfaceRunner: Send + Sync {
    /// Start the interface event loop.
    ///
    /// Returns when the interface shuts down cleanly (SIGINT/SIGTERM) or
    /// encounters a fatal error.
    async fn run(&self) -> Result<()>;
}
