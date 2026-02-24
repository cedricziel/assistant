//! Slack interface for the AI assistant.
//!
//! Receives messages via Slack Socket Mode and replies through the same
//! channel.  Each incoming message is routed through
//! `Orchestrator::run_turn_with_tools`.
//!
//! All tools remain available; use `allowed_channels` / `allowed_users`
//! and confirmation callbacks to control risky commands.

pub mod config;
pub mod runner;
pub mod skills;
pub mod tools;

pub use assistant_core::SlackConfig;
pub use runner::SlackInterface;
