//! Smoke test for `assistant-test-support`.
//!
//! Asserts that the crate compiles, the `prelude` module exists, and
//! `FixtureBuilder::new().build().await` returns a `Fixture`. As fakes
//! land in later phases of `workspace-test-coverage-floor`, this test
//! grows to assert their presence on the prelude.

use assistant_test_support::prelude::*;

#[tokio::test]
async fn fixture_builder_smoke() {
    let _fixture: Fixture = FixtureBuilder::new().build().await;
}

#[tokio::test]
async fn fixture_builder_default_smoke() {
    let _fixture = FixtureBuilder::default().build().await;
}
