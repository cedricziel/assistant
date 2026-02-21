//! Signal messenger interface for the AI assistant.
//!
//! This is a second full interface (parallel to the CLI REPL) that receives
//! messages via Signal and replies through the same channel.  It is
//! **feature-gated**: compile with `--features signal` to include it.
//!
//! # Architecture
//!
//! - A background `tokio::task` runs a presage listener loop.
//! - Each incoming Signal message is routed through `ReactOrchestrator::run_turn`.
//! - The `shell-exec` skill is **disabled** for this interface (see `SafetyGate`).
//! - Device linking is performed via `assistant signal link` (handled by the
//!   CLI binary, not this crate).
//!
//! # Feature-gating
//!
//! When compiled without `--features signal` this module compiles to an empty
//! stub so the rest of the workspace is unaffected.

pub mod config;
pub mod runner;

pub use config::SignalConfig;
pub use runner::SignalInterface;
