//! Signal messenger interface for the AI assistant.
//!
//! This is a second full interface (parallel to the CLI REPL) that receives
//! messages via Signal and replies through the same channel.  It is
//! **feature-gated**: compile with `--features signal` to include the presage
//! integration.
//!
//! # Architecture
//!
//! - Device linking is performed once via `assistant-signal link` (see
//!   [`link_device`]).
//! - A background tokio task runs a presage listener loop (see
//!   [`SignalInterface::run`]).
//! - Each incoming Signal message is routed through
//!   `ReactOrchestrator::run_turn_streaming`.
//! - The `shell-exec` skill is **disabled** for this interface via
//!   `SafetyGate`.
//!
//! # Feature-gating
//!
//! When compiled without `--features signal` this crate compiles to a stub
//! that returns informative errors, so the rest of the workspace is
//! unaffected.

pub mod config;
pub mod linker;
pub mod runner;

pub use assistant_core::SignalConfig;
pub use linker::link_device;
pub use runner::SignalInterface;
