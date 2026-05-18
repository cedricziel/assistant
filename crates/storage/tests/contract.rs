//! Cross-implementation contract tests for persistence traits.
//!
//! Each trait's contract test under `contract/<trait>.rs` runs the same
//! scenarios against every implementation of the trait. Drift between
//! impls fails CI.

mod contract {
    pub mod agent_store;
    pub mod attachment_store;
    pub mod command_event_store;
    pub mod conversation_store;
    pub mod log_store;
    pub mod push_subscription_store;
    pub mod slack_active_thread_store;
    pub mod trace_store;
    pub mod webhook_store;
}
