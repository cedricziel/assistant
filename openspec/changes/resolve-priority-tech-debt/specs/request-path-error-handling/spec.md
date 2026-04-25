## ADDED Requirements

### Requirement: no panics in web-ui request handlers

Non-test code in `crates/web-ui/src/api/`, `crates/web-ui/src/oauth/`, `crates/web-ui/src/auth.rs`, and `crates/web-ui/src/a2a/agent_store.rs` SHALL NOT use `.unwrap()`, `.expect()`, `panic!()`, `unreachable!()`, `todo!()`, or array/slice indexing that can panic on user input. Errors MUST be propagated with `?` and `anyhow::Context` (or a typed error envelope) and rendered as the standard `{"error": "..."}` JSON response with the correct HTTP status code.

#### Scenario: malformed input does not panic

- **WHEN** a request with malformed body, bad UUID, or missing field reaches an API handler in scope
- **THEN** the handler returns a 4xx response with the `{"error": "..."}` envelope and the server process keeps running

#### Scenario: downstream failure surfaces as 500

- **WHEN** a database, external service, or filesystem call fails inside an in-scope handler
- **THEN** the failure is propagated via `?` and surfaces as a 5xx response with the `{"error": "..."}` envelope, never as a panic

#### Scenario: clippy lint enforces the rule

- **WHEN** a developer adds a new `.unwrap()` or `.expect()` to non-test code in the in-scope files
- **THEN** clippy fails the build (via `#![deny(clippy::unwrap_used, clippy::expect_used)]` or equivalent module-level attributes)

### Requirement: no panics in storage migration and event paths

Non-test code in `crates/storage/src/migration.rs`, `crates/storage/src/conversation_events.rs`, `crates/storage/src/traces.rs`, and `crates/storage/src/webhooks.rs` SHALL NOT use `.unwrap()`, `.expect()`, or `panic!()` outside `Default`/`From` impls and constants. Errors MUST be returned as `Result<_, anyhow::Error>` (or the existing typed error) and the calling code SHALL handle them.

#### Scenario: migration failure is recoverable

- **WHEN** a migration step fails (e.g., schema mismatch, IO error)
- **THEN** the error is returned to the caller and surfaces as a non-panic startup failure with a logged context message

#### Scenario: malformed event row is logged and skipped or returned as error

- **WHEN** an event/trace/webhook row fails to deserialize
- **THEN** the failure is returned as `Err(...)` with context, never panics the worker thread

### Requirement: scope is non-test production paths only

Test code (`#[cfg(test)]` modules, `tests/` directories, fixture builders) is exempt from these requirements. The lints SHALL be applied per-module so test code can continue using `.unwrap()` for ergonomics.

#### Scenario: test code may use unwrap

- **WHEN** a `#[cfg(test)] mod tests` block uses `.unwrap()` or `.expect()`
- **THEN** the build succeeds
