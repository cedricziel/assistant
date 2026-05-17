## ADDED Requirements

### Requirement: Server-side keep-alive contract is enforced by automated tests

The web-ui's SSE response builder (`sse_response()` in `crates/web-ui/src/api/mod.rs`) and any direct `Sse::new(...)` call sites elsewhere SHALL apply `KeepAlive::default()`, AND a Rust integration test SHALL pin this behaviour end-to-end so future refactors cannot silently regress the keep-alive path.

#### Scenario: Slow stream still emits comment bytes within the keep-alive window

- **WHEN** an integration test connects to a streaming SSE endpoint
- **AND** the server is held in a deliberate >20-second silence (no semantic events emitted)
- **THEN** the raw byte stream observed by the test SHALL contain at least one keep-alive comment line (`:` + newline) within every 20-second window of that silence
- **THEN** the test SHALL fail if `KeepAlive::default()` is removed from any `Sse::new(...)` call site or if a future wrapper buffers the comments out

#### Scenario: Audit covers every Sse::new call site

- **WHEN** a developer adds a new SSE endpoint to `crates/web-ui`
- **THEN** the new `Sse::new(...)` call SHALL apply `.keep_alive(KeepAlive::default())`
- **THEN** if the new endpoint is wired through `sse_response()` (the recommended path), it inherits the contract automatically

### Requirement: Client byte-watchdog resets on any byte including comments

The Flutter client's `withHeartbeatTimeout` in `app/lib/api/api_client.dart` SHALL reset its timeout on any byte arriving on the underlying stream — including SSE comment bytes (`:` + newline), heartbeat bytes, partial event-frame bytes, and any other non-empty chunk. A Dart unit test SHALL pin this behaviour so future refactors cannot silently change it to "reset only on parsed events" or "reset only on data: lines".

#### Scenario: Stream of comment bytes alone does not trip the watchdog

- **WHEN** a Dart unit test drives `withHeartbeatTimeout` with a timeout of 60 seconds
- **AND** the source stream emits ONLY SSE comment bytes (`:\n`) at 10-second intervals for a total of 100 simulated seconds
- **THEN** the wrapped stream SHALL NOT emit a `TimeoutException`
- **THEN** the wrapped stream SHALL emit every comment-byte chunk it received, in order
