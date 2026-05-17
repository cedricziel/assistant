//! Composition layer for the `assistant` workspace's test fixtures.
//!
//! This crate is a thin re-export and composition layer. The actual fakes —
//! `InMemoryConversationStore`, `ScriptedLlmProvider`, `StubOrchestrationEngine`,
//! `FakeClock`, `InMemoryMessageBus`, and so on — live in their owning crates
//! next to their SQLite/production counterparts. The job of this crate is to:
//!
//! 1. Re-export those fakes through [`prelude`] so test files have one
//!    `use assistant_test_support::prelude::*;` line.
//! 2. Compose them via [`FixtureBuilder`] so tests can spin up a wired-up
//!    [`Fixture`] in a single chained call.
//!
//! ## Crate boundary rules
//!
//! - `assistant-test-support` MUST NOT appear in any crate's `[dependencies]`
//!   section. Only `[dev-dependencies]`. Enforced by
//!   `tests/workspace_test_support_lint.rs` at the repo root.
//! - Test-only types whose name starts with `InMemory`, `Scripted`, or `Stub`
//!   MUST NOT be constructed in non-test production code. Enforced by
//!   `tests/workspace_test_impls_in_prod.rs` at the repo root, with an explicit
//!   exempt list for documented production fallbacks
//!   (e.g. `InMemoryConversationBroadcaster`).
//!
//! See `openspec/changes/workspace-test-coverage-floor/specs/persistence-trait-symmetry/`.

pub mod fixture;
pub mod prelude;

pub use fixture::{Fixture, FixtureBuilder};
