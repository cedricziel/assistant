//! Fixture composition for `assistant` workspace tests.
//!
//! [`FixtureBuilder`] is a scaffold at this stage. Methods that wire `Clock`,
//! `LlmProvider`, `MessageBus`, and the persistence trait fakes land in later
//! phases of `openspec/changes/workspace-test-coverage-floor/` as their owning
//! crates land their in-memory implementations:
//!
//! - Phase 3 — `with_clock(FakeClock)`
//! - Phase 5 — `with_canned_llm_responses`, in-memory store wiring
//! - Phase 7 — `Arc<dyn OrchestrationEngine>` wired to `StubOrchestrationEngine`
//!
//! The public shape is fixed now so consumers can begin migrating to
//! `FixtureBuilder::new().build().await` calls and grow naturally.

/// Builder for a composed [`Fixture`]. Defaults wire every fake to a
/// sensible no-op initial state; per-field overrides come from `with_*`
/// methods that land in later phases.
#[derive(Default)]
pub struct FixtureBuilder {
    // Fields fill in as fakes land in later phases of
    // workspace-test-coverage-floor.
    _private: (),
}

impl FixtureBuilder {
    /// Start a new builder with all defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Materialise the [`Fixture`].
    ///
    /// At Phase 2 this is a no-op composition; later phases wire in real
    /// fakes (in-memory stores, scripted LLM provider, message bus, etc.).
    pub async fn build(self) -> Fixture {
        Fixture { _private: () }
    }
}

/// Composed test fixture. Carries the configured fakes once they land in
/// later phases of `workspace-test-coverage-floor`. At Phase 2 the type is
/// inhabited but carries no fields; tests construct it through
/// [`FixtureBuilder::build`].
pub struct Fixture {
    // Fields fill in as fakes land in later phases.
    _private: (),
}
