//! Signal messenger interface for the AI assistant.
//!
//! Connects to a [signal-cli-rest-api](https://github.com/bbernhard/signal-cli-rest-api)
//! daemon via WebSocket for inbound messages and REST for outbound replies.
//! Implements the standard [`ChannelAdapter`] / [`ChannelRunner`] pattern.

pub mod adapter;
pub mod config;
pub mod runner;

pub use adapter::SignalAdapter;
pub use assistant_core::SignalConfig;
pub use runner::SignalInterface;
