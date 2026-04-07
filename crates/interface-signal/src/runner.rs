//! Signal interface runner.
//!
//! [`SignalInterface`] delegates the receive/dispatch loop to
//! [`ChannelRunner`], which drives all messenger interfaces in this workspace.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::SignalConfig;
use assistant_runtime::{ChannelRunner, InterfaceRunner, Orchestrator};
use async_trait::async_trait;

use crate::adapter::SignalAdapter;

/// The Signal interface handle.
pub struct SignalInterface {
    config: SignalConfig,
    orchestrator: Arc<Orchestrator>,
}

impl SignalInterface {
    /// Create a new [`SignalInterface`].
    pub fn new(config: SignalConfig, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            config,
            orchestrator,
        }
    }

    /// Start the Signal listener loop via [`ChannelRunner`].
    pub async fn run(&self) -> Result<()> {
        let adapter = Arc::new(SignalAdapter::new(self.config.clone())?);
        ChannelRunner::new(adapter, self.orchestrator.clone())
            .run()
            .await
    }
}

#[async_trait]
impl InterfaceRunner for SignalInterface {
    async fn run(&self) -> Result<()> {
        self.run().await
    }
}

#[cfg(test)]
mod tests {
    use assistant_core::SignalConfig;

    #[test]
    fn allowlist_logic_empty_accepts_all() {
        let cfg = SignalConfig {
            allowed_senders: vec![],
            ..Default::default()
        };
        let sender = "some-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&sender);
        assert!(!blocked);
    }

    #[test]
    fn allowlist_logic_non_empty_blocks_unknown() {
        let cfg = SignalConfig {
            allowed_senders: vec!["allowed-uuid".to_string()],
            ..Default::default()
        };
        let unknown = "unknown-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&unknown);
        assert!(blocked);
    }

    #[test]
    fn allowlist_logic_non_empty_passes_known() {
        let cfg = SignalConfig {
            allowed_senders: vec!["allowed-uuid".to_string()],
            ..Default::default()
        };
        let known = "allowed-uuid".to_string();
        let blocked = !cfg.allowed_senders.is_empty() && !cfg.allowed_senders.contains(&known);
        assert!(!blocked);
    }
}
