## Context

Today each messenger interface bundles a large, opinionated Rust SDK:

| Interface  | SDK                | Lines of transitive dep code (approx)     |
| ---------- | ------------------ | ----------------------------------------- |
| Slack      | `slack-morphism`   | ~15k                                      |
| Mattermost | `mattermost_api`   | ~8k                                       |
| Matrix     | `matrix-sdk`       | ~80k+ (full protocol stack, SQLite state) |
| Nextcloud  | `reqwest` directly | — (already thin)                          |
| Signal     | `presage`          | ~40k (crypto + protocol)                  |

These SDKs each own the HTTP transport, impose their own async connection managers, and expose platform-idiosyncratic event models. Adding a new channel means learning a new SDK. Testing requires mocking at the SDK level (no standard seam). The `InterfaceRunner::run()` surface is uniform but everything underneath it is bespoke.

OpenFang's `ChannelAdapter` trait (RightNow-AI/openfang) demonstrates that a simple three-method async trait — `start() → Stream<ChannelMessage>`, `send()`, `stop()` — is sufficient to host 40+ channel adapters, all implemented as thin `reqwest` HTTP clients.

## Goals / Non-Goals

**Goals:**

- Define a `ChannelAdapter` trait + unified `ChannelMessage`/`ChannelContent`/`ChannelUser` types in `assistant-core` (or a new `assistant-channels` crate).
- Replace `slack-morphism` with a thin `reqwest` + `tokio-tungstenite` Slack Socket Mode client.
- Replace `mattermost_api` with a thin `reqwest` + `tokio-tungstenite` Mattermost WebSocket + REST client.
- Replace `matrix-sdk` with a thin `reqwest` Matrix Client-Server spec implementation.
- Align `interface-nextcloud` (already uses `reqwest`) to the new trait.
- Wrap `interface-signal` (protocol-level) behind the trait without changing its presage dependency.
- Keep `InterfaceRunner::run()` as the public entry point — adapters are an internal detail.
- Make every adapter testable via `wiremock` (already in workspace).

**Non-Goals:**

- Supporting every Matrix feature (E2EE, VoIP, cross-signing) — plain text messaging only.
- Replacing `presage` in the Signal adapter.
- Adding new channel types (Discord, Telegram, etc.) — that is a follow-on.
- Changing the orchestrator interface or `ToolHandler` contract.

## Decisions

### 1. Trait location: `assistant-core` vs new `assistant-channels` crate

**Decision**: Add to `assistant-core`.

**Rationale**: The trait and unified types need to be visible to all interface crates and to `assistant-runtime` (dispatch layer). A new crate adds a dependency edge but no meaningful encapsulation benefit at this stage. `assistant-core` already owns `ToolHandler`, `MessageBus`, and `Interface` enum — channel types belong in the same namespace.

**Alternative considered**: New `assistant-channels` crate. Rejected because it would require all 5 interface crates to add a new dep, and `assistant-runtime` would need it too, adding churn with no encapsulation win.

---

### 2. WebSocket library: `tokio-tungstenite` vs `async-tungstenite`

**Decision**: `tokio-tungstenite`.

**Rationale**: Already used transitively in the workspace (via `matrix-sdk`). Tokio-native, well-maintained, minimal API surface. `async-tungstenite` supports multiple runtimes but we are Tokio-only throughout.

---

### 3. `ChannelAdapter` stream vs callback

**Decision**: `start()` returns `Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>`.

**Rationale**: A `Stream` composes cleanly with `tokio::select!`, allows backpressure, and is mockable. Callback-based designs (as used in current Slack/Mattermost) require `Arc<dyn Fn>` plumbing and make testing harder.

---

### 4. Dispatch layer placement

**Decision**: Each `InterfaceRunner` implementation owns its dispatch loop: it drives `ChannelAdapter::start()`, calls `Orchestrator::run_turn_with_tools()` per message, and calls `ChannelAdapter::send()` with the result.

**Rationale**: Keeps the runtime unaware of channel specifics. A shared dispatch crate is a tempting abstraction but would introduce coupling between `assistant-runtime` and the new channel types before the pattern is proven. Can be extracted later.

---

### 5. Matrix without `matrix-sdk`

**Decision**: Implement Matrix as plain long-poll sync (`GET /_matrix/client/v3/sync`) with `reqwest`. Persist the `next_batch` token to a file (or SQLite) between restarts.

**Rationale**: `matrix-sdk` provides a full Matrix SDK including E2EE, cross-signing, and a full state store. We only need to receive plain-text messages and send replies. The Client-Server spec sync endpoint is well-documented and stable. E2EE is explicitly out of scope.

**Alternative considered**: Keep `matrix-sdk` but wrap it behind the trait. Rejected because it defeats the goal of thin clients and keeps the 80k+ line transitive dep.

---

### 6. Breaking change: unified message types vs per-interface structs

**Decision**: Introduce `ChannelMessage`/`ChannelContent`/`ChannelUser` as the canonical event types. Per-interface structs (`SlackIncomingEvent`, etc.) become private implementation details inside each adapter.

**Rationale**: The unified types are the whole point — they make adding channels cheap and testing uniform.

## Risks / Trade-offs

- [Matrix E2EE rooms will not work] → Mitigation: document that the Matrix adapter requires unencrypted rooms. Add a warning at startup if the homeserver reports E2EE for a room.
- [Slack Socket Mode reconnection logic is non-trivial] → Mitigation: implement exponential backoff (1s → 60s cap, jitter) with a background reconnect task, matching OpenFang's pattern.
- [tokio-tungstenite adds a direct dep where it was previously transitive] → Mitigation: low risk; it is stable and already in the dep graph.
- [Mattermost WebSocket token refresh] → Mitigation: re-authenticate on 401 responses and reconnect.
- [Signal presage coupling remains] → Mitigation: Signal adapter implements `ChannelAdapter` via a thin wrapper; no change to presage usage.

## Migration Plan

1. Add `ChannelAdapter` trait + types to `assistant-core` (no breaking change — additive).
2. Implement new thin clients alongside existing ones (feature-flag or separate module).
3. Wire new clients into `InterfaceRunner` implementations, remove old SDK deps.
4. Delete old SDK wrappers and per-interface event structs.
5. Run full test suite (`make test`) and integration tests (`make test-integration`).
6. Remove old SDK entries from `Cargo.toml`.

No database migrations required. No config format changes.

## Open Questions

- Should `ChannelAdapter` expose a `capabilities() → ChannelCapabilities` method (threads, reactions, file upload)? Could let the dispatch layer skip unsupported operations gracefully. Deferred for now; adapters can no-op optional methods.
- Should conversation-UUID keying (channel+thread → UUID) live in the adapter or the dispatch layer? Currently in the interface crates. Likely belongs in the dispatch layer for reuse.
