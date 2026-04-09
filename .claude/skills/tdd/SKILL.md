---
name: tdd
description: >
  Test Driven Development discipline for this workspace. Use whenever writing new features,
  fixing bugs, or refactoring — the test must be written and confirmed failing before any
  implementation code is added.
---

## Principle

**Always start with a failing test.** No implementation code may be written until a test
that exercises the desired behaviour exists and is confirmed to fail for the right reason.

## The TDD Cycle

1. **Red** — Write the smallest test that captures the requirement. Run it; confirm it fails
   (and fails for the expected reason, not a compile error or unrelated panic).
2. **Green** — Write the minimum production code required to make the test pass. Resist the
   urge to add more than needed.
3. **Refactor** — Clean up both test and production code. Re-run the test suite after every
   change to stay green.

Repeat the cycle for each new behaviour.

## Rules

1. **Test first, always.** If you are about to write a function, struct, or module, write the
   test file (or test module) first.
2. **Confirm red before green.** Run `make test` (or the narrower `cargo test -p <crate>
<test_name>`) and verify the new test fails before writing implementation.
3. **One failing test at a time.** Do not write multiple new tests before making the first
   one pass. Keep the red/green cycle tight.
4. **Minimal implementation.** In the Green step, write only enough code to pass the current
   failing test — nothing more.
5. **Refactor under green.** Only refactor when all tests are passing. Never refactor and add
   features simultaneously.
6. **Tests are first-class code.** Apply the same naming, clarity, and style standards to
   test code as to production code.

## Rust-Specific Guidance

- Unit tests live in `#[cfg(test)] mod tests { ... }` at the bottom of the file under test.
- Use `#[tokio::test]` for async tests.
- Use `StorageLayer::new_in_memory()` for test databases — no disk I/O, fully migrated.
- Use `wiremock` for HTTP mocking (provider API responses).
- Integration tests live in `crates/integration-tests/` and require a running environment;
  run with `make test-integration`.
- For a new builtin tool, write the unit test in the handler file _before_ implementing
  `ToolHandler::run()`.

## Workflow

When picking up a task:

1. Identify the smallest observable behaviour to implement next.
2. Write a test for that behaviour in the appropriate `#[cfg(test)] mod tests` block.
3. Run `cargo test -p <crate> <test_name> -- --nocapture` and confirm it is **red**.
4. Implement only what is needed to turn it **green**.
5. Run `make test` to ensure no regressions.
6. Refactor if needed; stay green throughout.
7. Commit (using a semantic commit message) and move to the next behaviour.
