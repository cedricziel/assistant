//! Mattermost messenger interface adapter.
//!
//! WebSocket client, adapter, runner, and platform tools.

mod adapter;
pub(crate) mod client;
pub(crate) mod config;
pub mod runner;
mod tools;

pub use runner::MattermostInterface;
