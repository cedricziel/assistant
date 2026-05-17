//! Smoke test for `assistant-test-support`.
//!
//! Asserts that the crate compiles, the `prelude` module exists, and
//! `FixtureBuilder::new().build().await` returns a `Fixture`. As fakes
//! land in later phases of `workspace-test-coverage-floor`, this test
//! grows to assert their presence on the prelude.

use std::sync::Arc;
use std::time::Duration;

use assistant_test_support::prelude::*;

#[tokio::test]
async fn fixture_builder_smoke() {
    let _fixture: Fixture = FixtureBuilder::new().build().await;
}

#[tokio::test]
async fn fixture_builder_default_smoke() {
    let _fixture = FixtureBuilder::default().build().await;
}

#[test]
fn prelude_exports_fake_clock() {
    // The prelude must surface FakeClock + Clock + SystemClock so test
    // code anywhere in the workspace can `use assistant_test_support::prelude::*;`
    // and immediately work with the time abstraction.
    let fake = FakeClock::new_at_seed();
    let t0 = fake.now();
    fake.advance(Duration::from_secs(30));
    let t1 = fake.now();
    assert_eq!((t1 - t0).num_seconds(), 30);

    // Clock is dyn-compatible — both impls coerce to Arc<dyn Clock>.
    let _system: Arc<dyn Clock> = Arc::new(SystemClock);
    let _fake_dyn: Arc<dyn Clock> = Arc::new(fake);
}
