## ADDED Requirements

### Requirement: messenger runners separate I/O from dispatch

Messenger interface adapters SHALL separate their WebSocket / long-poll I/O loop from their per-event dispatch logic. This applies to every messenger under `crates/interfaces/src/{slack, mattermost, matrix, nextcloud, signal}/`.

The I/O loop (`runner.rs`) SHALL be limited to:

- opening and maintaining the transport (WebSocket, long-poll cursor).
- decoding raw frames into typed event enums.
- forwarding each event to the dispatch function.
- handling reconnect / backoff.

The dispatch function (`dispatch.rs`, or a sibling module) SHALL be a
pure-ish async function with the shape:

```rust
pub async fn handle_event(
    event: <Messenger>Event,
    deps: &<Messenger>RunnerDeps,
) -> anyhow::Result<RunnerAction>;
```

where `<Messenger>RunnerDeps` bundles the trait-typed dependencies
(`Arc<dyn OrchestrationEngine>`, `Arc<dyn Clock>`, the messenger's
HTTP client, storage handles, etc.).

#### Scenario: dispatch function is unit-testable

- **WHEN** a test fixture constructs `<Messenger>Event` from a
  `serde_json::json!` payload, builds a `<Messenger>RunnerDeps`
  carrying hand-rolled fakes, and calls `handle_event`
- **THEN** the function returns a `RunnerAction` (or executes
  through the fakes) that the test can assert without opening a
  real WebSocket

#### Scenario: I/O loop stays slim

- **WHEN** the runner's WebSocket loop is inspected
- **THEN** all decoded events are dispatched via the `handle_event`
  function; per-event business logic (orchestrator invocation,
  reaction posting, attachment upload) does not appear inline in
  the loop

### Requirement: `RunnerAction` describes side effects declaratively

Each messenger module SHALL define a `RunnerAction` enum that
describes the side effects produced by a single dispatch invocation:

- `NoOp` — the event was ignored (e.g., bot author, channel not allowed).
- `Reply { ... }` — post a message in a channel/thread.
- `Reaction { ... }` — add or remove an emoji reaction.
- `UploadAttachment { ... }` — upload bytes to the messenger.
- platform-specific variants as needed.

Dispatch tests SHALL assert on `RunnerAction` rather than on the
side effect's HTTP request. Side-effect execution itself MAY be
exercised by a separate, smaller set of integration tests that use
`wiremock`.

#### Scenario: dispatch test asserts on action

- **WHEN** a unit test runs `handle_event(SlackEvent::Message { ... })`
- **THEN** it asserts `result == RunnerAction::Reply { channel, text, ... }`
  rather than asserting on an outbound HTTP request

### Requirement: bot-author filter and channel allowlist are unit-tested

The dispatch function SHALL exercise the following guards inside
`handle_event` and SHALL be unit-tested for each:

- the event author is the bot itself → `RunnerAction::NoOp`.
- the channel is not on the per-messenger allowlist → `NoOp`.
- the event is a thread reply to a thread the assistant did not
  initiate → `NoOp` (or platform-specific behavior).
- the event is a duplicate (already-seen `event_id`) → `NoOp`.

#### Scenario: bot loop is prevented

- **WHEN** a test constructs an event whose author equals the bot
  user id stored in `RunnerDeps`
- **THEN** `handle_event` returns `RunnerAction::NoOp` and does not
  invoke the orchestrator fake

### Requirement: CLI REPL applies the same pattern

The CLI REPL SHALL separate its input loop from its per-command dispatch. A `dispatch_command(cmd, deps)` function SHALL handle each command's logic in `crates/interface-cli/src/main.rs` and SHALL be unit-tested for each `Command` variant.

#### Scenario: CLI subcommand is unit-testable

- **WHEN** a test calls `dispatch_command(Command::Persona { ... },
deps)`
- **THEN** the test can assert on the resulting state without
  spawning a child process or reading from stdin
