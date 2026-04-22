//! Matrix messenger interface adapter.
//!
//! Long-poll `/sync` client, adapter, runner, and platform tools.

mod adapter;
pub(crate) mod client;
pub(crate) mod config;
pub mod runner;
mod tools;

pub use runner::MatrixInterface;
