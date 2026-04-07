//! Signal `ChannelAdapter` stub.
//!
//! Signal's presage library uses non-`Send` types internally, which prevents
//! wrapping the receive loop as a `Send` stream.  The full Signal integration
//! lives in [`runner::SignalInterface`], which handles presage directly.
//!
//! This module exists so the Signal interface is listed under `ChannelAdapter`
//! in the public API; in practice callers use `SignalInterface::run()` directly.

use std::pin::Pin;

use anyhow::Result;
use assistant_core::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser};
use async_trait::async_trait;
use futures::stream::Stream;

use crate::config::SignalConfig;

/// Stub adapter — conformance only.  The real loop is in `SignalInterface::run`.
pub struct SignalAdapter {
    #[allow(dead_code)]
    config: SignalConfig,
    stop_tx: tokio::sync::watch::Sender<bool>,
}

impl SignalAdapter {
    pub fn new(config: SignalConfig) -> Self {
        let (stop_tx, _) = tokio::sync::watch::channel(false);
        Self { config, stop_tx }
    }
}

#[async_trait]
impl ChannelAdapter for SignalAdapter {
    fn name(&self) -> &str {
        "signal"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Signal
    }

    async fn start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send + 'static>>> {
        anyhow::bail!(
            "Signal streaming is not supported via ChannelAdapter::start(). \
             Use SignalInterface::run() instead."
        )
    }

    async fn send(&self, _user: &ChannelUser, _content: ChannelContent) -> Result<()> {
        anyhow::bail!("Signal send is not supported via ChannelAdapter. Use the runner.")
    }

    async fn stop(&self) -> Result<()> {
        let _ = self.stop_tx.send(true);
        Ok(())
    }
}
