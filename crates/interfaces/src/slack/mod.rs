//! Slack interface adapter.
//!
//! Socket Mode runner, thin API client, ambient tools, and configuration helpers.

mod adapter;
pub(crate) mod client;
pub(crate) mod config;
pub mod runner;
pub(crate) mod skills;
mod tools;

pub use runner::SlackInterface;
