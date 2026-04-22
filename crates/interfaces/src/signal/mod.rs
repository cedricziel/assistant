//! Signal messenger interface adapter.
//!
//! Thin client against signal-cli-rest-api via WebSocket and REST.

pub mod adapter;
pub(crate) mod config;
pub mod runner;

pub use adapter::SignalAdapter;
pub use runner::SignalInterface;
