//! Mattermost interface for the AI assistant.
//!
//! Receives messages via Mattermost WebSocket events and replies through the
//! same channel.  Each incoming message is routed through
//! `Orchestrator::run_turn_with_tools`.
//!
//! The `shell-exec` skill is **disabled** for this interface via `SafetyGate`.

pub mod config;
pub mod runner;
pub mod tools;

pub use assistant_core::MattermostConfig;
pub use runner::MattermostInterface;
