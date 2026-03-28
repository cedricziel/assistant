# Data Model: Matrix Interface

**Branch**: `002-matrix-interface` | **Date**: 2026-03-28

## Entities

### MatrixConfig

Defined in `crates/core/src/types.rs`, embedded in `AssistantConfig` as `pub matrix: Option<MatrixConfig>`.

| Field              | Type             | Required              | Description                                                                                                       |
| ------------------ | ---------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `homeserver_url`   | `Option<String>` | Yes (via env)         | Base URL of the Matrix homeserver, e.g. `https://matrix.org`. Fallback: `MATRIX_HOMESERVER_URL` env var.          |
| `username`         | `Option<String>` | Yes (via env)         | Full Matrix user ID for the bot, e.g. `@assistant:example.com`. Fallback: `MATRIX_USERNAME` env var.              |
| `password`         | `Option<String>` | One of password/token | Bot account password (used for initial login; session is persisted to disk). Fallback: `MATRIX_PASSWORD` env var. |
| `access_token`     | `Option<String>` | One of password/token | Pre-issued Matrix access token (skips password login). Fallback: `MATRIX_ACCESS_TOKEN` env var.                   |
| `device_id`        | `Option<String>` | No                    | Device ID for session restoration. Auto-generated on first login if omitted.                                      |
| `state_store_path` | `Option<String>` | No                    | Path for `matrix-sdk` SQLite state store. Default: `~/.assistant/matrix-state/`.                                  |
| `allowed_rooms`    | `Vec<String>`    | No                    | Room IDs to accept messages from. Empty list = all rooms.                                                         |
| `allowed_users`    | `Vec<String>`    | No                    | Matrix user IDs allowed to trigger the bot. Empty list = all users.                                               |

**Validation rules**:

- At least one of `homeserver_url` or `MATRIX_HOMESERVER_URL` must be present at runtime.
- At least one of `username` or `MATRIX_USERNAME` must be present at runtime.
- At least one of (`password` / `MATRIX_PASSWORD`) or (`access_token` / `MATRIX_ACCESS_TOKEN`) must be present at runtime.

### ConversationSession (runtime, not persisted)

Held in an `Arc<Mutex<LruCache<String, Uuid>>>` within `MatrixInterface`.

| Field | Type     | Description                                            |
| ----- | -------- | ------------------------------------------------------ |
| key   | `String` | Matrix room ID (canonical, e.g. `!abc123:example.com`) |
| value | `Uuid`   | Conversation ID passed to `Orchestrator::submit_turn`  |

- **Capacity**: 10,000 entries (LRU eviction), matching Mattermost pattern.
- **Lifetime**: In-memory only; resets on process restart (new conversation UUID per room on next message).

### Interface enum addition

In `crates/core/src/types.rs`, add:

```rust
Interface::Matrix
```

This variant is used when calling `Orchestrator::submit_turn` and in `run_worker_filtered("matrix-worker", Some("Matrix"))`.

## State Transitions

```text
Bot starts
  └─> Login / RestoreSession
        └─> Sync loop running
              ├─> RoomMessage event received
              │     ├─> Filtered out (self, not-allowed room/user) → discard
              │     └─> Dispatched → Orchestrator::submit_turn
              │                         └─> Reply sent to Matrix room
              └─> Sync error
                    └─> Exponential backoff → reconnect
Bot stops (SIGINT/SIGTERM)
  └─> Shutdown channel triggered → sync loop exits cleanly
```

## Dependencies on Existing Entities

- `AssistantConfig` (in `assistant-core`) — gains `pub matrix: Option<MatrixConfig>` field.
- `Interface` enum (in `assistant-core`) — gains `Matrix` variant.
- `Orchestrator` (in `assistant-runtime`) — consumed via `Arc<Orchestrator>`; no changes needed.
