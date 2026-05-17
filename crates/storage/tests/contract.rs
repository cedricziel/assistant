//! Cross-implementation contract tests for persistence traits.
//!
//! Each trait's contract test under `contract/<trait>.rs` runs the same
//! scenarios against every implementation of the trait. Drift between
//! impls fails CI.

mod contract {
    pub mod conversation_store;
}
