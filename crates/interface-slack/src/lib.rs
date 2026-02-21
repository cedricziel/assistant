//! Slack interface for the AI assistant.
//!
//! This is a third full interface (parallel to the CLI REPL and Signal) that
//! receives messages via Slack and replies through the same channel.  It is
//! **feature-gated**: compile with `--features slack` to include the
//! slack-morphism Socket Mode integration.
//!
//! # Architecture
//!
//! - A background tokio task runs a Socket Mode WebSocket listener (see
//!   [`SlackInterface::run`]).
//! - Each incoming Slack message or app-mention is routed through
//!   `ReactOrchestrator::run_turn_streaming`.
//! - The `shell-exec` skill is **disabled** for this interface via
//!   `SafetyGate`.
//!
//! # Feature-gating
//!
//! When compiled without `--features slack` this crate compiles to a stub
//! that returns informative errors, so the rest of the workspace is
//! unaffected.

pub mod config;
pub mod runner;

pub use assistant_core::SlackConfig;
pub use runner::SlackInterface;
