# Research: Matrix Interface

**Branch**: `002-matrix-interface` | **Date**: 2026-03-28

## Decision 1: Matrix Client SDK

**Decision**: Use `matrix-sdk` (the official Matrix.org Rust SDK)
**Rationale**: `matrix-sdk` is the only production-grade, actively maintained Matrix client library for Rust. It provides async/await via tokio, event handler registration, automatic sync management, room membership handling, and a clean high-level API. The low-level alternative (`ruma`) requires assembling everything manually and is not appropriate for a bot integration.
**Alternatives considered**:

- `ruma` — low-level Matrix protocol types only; no client state machine; rejected as too much boilerplate
- Raw HTTP/WebSocket — would require reimplementing sync, pagination, and retry; rejected
- `matrix-sdk` at a pinned version — chosen; latest stable release (0.7.x series as of early 2026)

## Decision 2: Authentication Strategy

**Decision**: Access token authentication (pre-issued long-lived token or session persisted to disk after first password login)
**Rationale**: The project spec explicitly defers interactive login to v2. Access tokens are the standard bot authentication method in Matrix homeservers (both Synapse and Dendrite). `matrix-sdk` supports `Client::restore_session()` to resume from a saved session file, and `Client::login_username()` for initial password-based login that produces a reusable token.
**Alternatives considered**:

- Password login on every start — rejected; insecure and causes unnecessary device creation
- Device-based SSO / OIDC — rejected as out of scope for v1
- Bearer token via env var only — acceptable but less ergonomic; config file is preferred with env var fallback (matching Mattermost/Slack pattern)

## Decision 3: Event Subscription Model

**Decision**: `matrix-sdk` sync loop with `client.add_event_handler()` for `OriginalSyncRoomMessageEvent`
**Rationale**: `matrix-sdk` abstracts Matrix `/sync` polling. Registering typed event handlers is idiomatic and handles event deduplication automatically. For our use case (listening to room messages) a single event handler type is sufficient.
**Alternatives considered**:

- Raw `/sync` HTTP polling — requires manual state tracking; rejected
- `matrix-sdk`'s `SyncSettings` with a custom filter — considered; a room message filter reduces bandwidth but adds complexity; deferred to v2 optimization

## Decision 4: Conversation Context Key

**Decision**: Key conversations by `room_id` (one session per room)
**Rationale**: In Matrix every room has a unique room ID, and DMs are also regular rooms. Keying by `room_id` gives the same isolation guarantee as the Mattermost `(channel_id, root_post_id)` strategy, but simpler since Matrix does not have thread roots in the same sense. Thread support (reply chains as sub-contexts) is deferred to v2.
**Alternatives considered**:

- `(room_id, thread_root_event_id)` — more granular but thread IDs are optional in Matrix; complicates v1 significantly; deferred
- Per-user-per-room keys — would split conversations by user within the same room; not the desired UX

## Decision 5: Bot Self-Message Filtering

**Decision**: Fetch the bot's own `user_id` on startup via `client.user_id()` after login; filter out events where `event.sender == bot_user_id`
**Rationale**: `matrix-sdk` exposes `Client::user_id()` after a successful login. This is the same approach as the Mattermost interface (fetch `/users/me` on startup). Prevents infinite reply loops.
**Alternatives considered**:

- Set a `sender_localpart` filter in `/sync` — possible but requires filter setup on homeserver; less portable
- Trust that the SDK deduplicates — insufficient; the SDK delivers all room events including own messages

## Decision 6: Session Persistence

**Decision**: Use `matrix-sdk`'s `SqliteCryptoStore` / `SqliteStateStore` for session persistence in a configurable state directory (default: `~/.assistant/matrix-state/`)
**Rationale**: Without state persistence the bot re-downloads all room history on every restart (initial sync). `matrix-sdk` provides SQLite-backed stores. Since encryption is out of scope for v1, only the state store (room membership, sync token) is strictly needed; the crypto store is added as a no-op placeholder.
**Alternatives considered**:

- In-memory store — simpler but causes full re-sync on restart; rejected for production use
- Custom file store — unnecessary when `matrix-sdk` provides one

## Decision 7: Reconnect / Error Handling Strategy

**Decision**: Exponential backoff (1 s → 2 s → … → 60 s cap) on sync errors; `matrix-sdk` handles rate-limit retries internally
**Rationale**: Matches the Mattermost reconnect pattern. `matrix-sdk`'s `Client::sync()` already retries transient network errors, but explicit backoff in our wrapper loop handles server-side resets and fatal errors.
**Alternatives considered**:

- Fixed 5 s retry — simpler but hammers the server during sustained outage; rejected

## Decision 8: Allowed Rooms Filtering

**Decision**: `allowed_rooms: Vec<String>` (room IDs) + `allowed_users: Vec<String>` in `MatrixConfig`, checked in the event handler before dispatching to the orchestrator
**Rationale**: Same pattern as `allowed_channels` and `allowed_users` in Mattermost/Slack configs. Empty list = accept all (permissive default).
**Alternatives considered**:

- Room-name-based filtering — brittle; room names can change; room IDs are canonical in Matrix
- Server-side join control — requires homeserver configuration outside our scope

## Resolved NEEDS CLARIFICATION Items

None were present in the spec. All design choices above were made as informed defaults based on Matrix protocol knowledge and existing project patterns.
