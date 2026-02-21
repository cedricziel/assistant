//! Mattermost interface for the AI assistant.
//!
//! This is a fourth full interface (parallel to the CLI REPL, Signal, and
//! Slack) that receives messages via Mattermost and replies through the same
//! channel.  It is **feature-gated**: compile with `--features mattermost` to
//! include the mattermost_api integration.
//!
//! # Architecture
//!
//! - A background tokio task runs a Mattermost WebSocket event listener (see
//!   [`MattermostInterface::run`]).
//! - Each incoming `posted` event is routed through
//!   `ReactOrchestrator::run_turn_streaming`.
//! - The `shell-exec` skill is **disabled** for this interface via
//!   `SafetyGate`.
//!
//! # Feature-gating
//!
//! When compiled without `--features mattermost` this crate compiles to a stub
//! that returns informative errors, so the rest of the workspace is
//! unaffected.

pub mod config;
pub mod runner;

pub use assistant_core::MattermostConfig;
pub use runner::MattermostInterface;
