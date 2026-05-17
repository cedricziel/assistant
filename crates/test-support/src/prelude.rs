//! Prelude — single-import re-exports for the most-used fixture types and
//! workspace fakes.
//!
//! Test code should prefer
//!
//! ```ignore
//! use assistant_test_support::prelude::*;
//! ```
//!
//! over individual imports. As fakes land in their owning crates in later
//! phases of `openspec/changes/workspace-test-coverage-floor/`, they are
//! re-exported here:
//!
//! - Phase 3 — `FakeClock` from `assistant_core::clock`
//! - Phase 5 — `InMemory*Store` types from `assistant_storage` and
//!   `ScriptedLlmProvider` from `assistant_llm_provider`
//! - Phase 7 — `StubOrchestrationEngine`, `StubToolDispatcher`,
//!   `InMemorySkillCatalog`

pub use crate::fixture::{Fixture, FixtureBuilder};
