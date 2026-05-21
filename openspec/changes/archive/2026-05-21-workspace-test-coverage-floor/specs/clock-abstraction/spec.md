## ADDED Requirements

### Requirement: `Clock` trait in `assistant-core`

`assistant-core` SHALL define a `Clock` trait abstracting wall-clock and
monotonic time access:

```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
    fn now_instant(&self) -> std::time::Instant;
}
```

`assistant-core` SHALL also export a `SystemClock` implementation that
wraps `chrono::Utc::now()` and `std::time::Instant::now()` for
production use.

#### Scenario: production code receives `SystemClock`

- **WHEN** a production binary (CLI, web-ui, MCP server) constructs a
  component that needs the current time
- **THEN** it passes `Arc::new(SystemClock) as Arc<dyn Clock>` (or
  defaults to it via a builder)

### Requirement: `FakeClock` for tests

`assistant-core` SHALL expose a `FakeClock` test helper that allows
seeding and advancing a virtual time. `FakeClock` SHALL live behind
`#[cfg(any(test, feature = "test-support"))]` so production binaries
cannot accidentally depend on it.

`FakeClock` SHALL support:

- construction with a seed `DateTime<Utc>`
- `advance(Duration)` to move time forward
- thread-safe read access (interior mutability)

#### Scenario: test seeds and advances time

- **WHEN** a test constructs `FakeClock::new(seed)` and later calls
  `fake.advance(Duration::from_secs(60))`
- **THEN** subsequent calls to `fake.now()` return `seed + 60s`

### Requirement: direct `Utc::now()` is banned in non-test production code

Non-test production code under `crates/*/src/` SHALL NOT call
`chrono::Utc::now()`, `chrono::Local::now()`, or
`std::time::SystemTime::now()` directly. All current-time reads MUST go
through an injected `Arc<dyn Clock>`.

Exemptions:

- `assistant-core::clock::SystemClock` — the canonical wrapper.
- Top-level binary entry points (`main.rs`) when constructing the root
  `SystemClock` instance.

#### Scenario: workspace lint test enforces the ban

- **WHEN** `cargo test -p assistant tests::workspace_clock_lint` runs
- **THEN** the test greps all non-test `.rs` files in `crates/*/src/`
  for `Utc::now\(\)` / `SystemTime::now\(\)` and fails if any match
  occurs outside the exempt paths

#### Scenario: test code is allowed to call `Utc::now()`

- **WHEN** a `#[cfg(test)] mod tests` or a `tests/*.rs` file calls
  `Utc::now()` directly
- **THEN** the workspace lint test ignores the call (test code is
  exempt)

### Requirement: time-sensitive components accept `Arc<dyn Clock>`

The following components SHALL accept `Arc<dyn Clock>` at construction:

- `assistant_auth::jwt::JwtManager` (token issuance + expiry checks).
- `assistant_auth::oauth2::device::DeviceCodeManager` (device-code TTL).
- `assistant_auth::api_keys` lifecycle (`expires_at`, `last_used_at`).
- `assistant_runtime::scheduler::Scheduler` (firing decisions).
- `assistant_runtime::title_generator::TitleGeneratorWorker` (debounce).
- `assistant_runtime::memory_indexer` (indexing cadence).
- `assistant_runtime::compaction` (eviction timestamps).
- `assistant_llm_provider::retry` (backoff between attempts).
- `assistant_storage::conversation_events`, `traces`, `metrics` (row
  timestamps).
- `assistant_bus_nats` (heartbeat / TTL).

Each component SHALL provide a constructor that defaults to
`SystemClock` so production call sites do not need to change.

#### Scenario: JWT expiry verification under FakeClock

- **WHEN** a JWT is issued with `JwtManager::new_with_clock(fake_clock, ...)`
  at `t = 0` with expiry `60s`, and `fake_clock.advance(Duration::from_secs(61))`
- **THEN** verification of that JWT fails with the expired-token error

#### Scenario: scheduler fires under FakeClock

- **WHEN** a scheduler is configured with a task at `t = 60s` and the
  test advances the fake clock to `t = 60s`
- **THEN** the scheduler dispatches the task on its next tick
