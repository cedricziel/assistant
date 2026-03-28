//! Matrix interface for the AI assistant.
//!
//! Receives messages via Matrix sync and replies through the same room.
//! Each incoming message is routed through `Orchestrator::run_turn_with_tools`.
//!
//! All tools remain available; use `allowed_rooms` / `allowed_users` to
//! control which rooms and users can trigger the assistant.

pub mod config;
pub mod runner;
pub mod tools;

pub use assistant_core::MatrixConfig;
pub use runner::MatrixInterface;
