//! Signal interface runner.
//!
//! Without `--features signal` this provides a no-op `SignalInterface` that
//! returns an informative error when started.
//!
//! To enable the real Signal integration, see Cargo.toml for the required
//! presage git dependencies and rebuild with `--features signal`.

use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::ReactOrchestrator;

use crate::config::SignalConfig;

/// The Signal interface handle.
pub struct SignalInterface {
    #[allow(dead_code)]
    config: SignalConfig,
    #[allow(dead_code)]
    orchestrator: Arc<ReactOrchestrator>,
}

impl SignalInterface {
    /// Create a new `SignalInterface`.
    ///
    /// Call [`run`] to start the listener loop (requires `--features signal`).
    pub fn new(config: SignalConfig, orchestrator: Arc<ReactOrchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Signal listener loop.
    ///
    /// Without `--features signal` this always returns an informative error
    /// explaining how to enable the feature.
    pub async fn run(&self) -> Result<()> {
        #[cfg(not(feature = "signal"))]
        anyhow::bail!(
            "The Signal interface requires recompiling with `--features signal`.\n\
             Add the presage git dependencies to Cargo.toml and rebuild:\n\
             \n\
             cargo build --workspace --features assistant-interface-signal/signal\n\
             \n\
             See crates/interface-signal/Cargo.toml for the required dependencies."
        );

        // Real implementation would go here, gated by #[cfg(feature = "signal")]
        // using the presage crate for Signal protocol support.
        #[cfg(feature = "signal")]
        {
            // Presage-based listener — add implementation once the presage
            // dependency is configured and the feature is enabled.
            anyhow::bail!("Signal feature is defined but presage integration is not yet wired up. Implement in runner.rs.")
        }
    }
}
