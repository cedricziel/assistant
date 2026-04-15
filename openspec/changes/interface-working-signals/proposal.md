## Why

Users have no visual feedback that the agent received their message and is actively working on a response — the interface appears silent until a full reply arrives. There are two distinct moments to signal:

1. **Message received / queued** — the runner has the message but another turn is already in progress. An ⏳ hourglass should appear immediately so the user knows the message was received.
2. **Processing started** — the turn lock is acquired and the agent is actively running. The hourglass clears, a 👀 reaction or typing indicator appears.

Slack additionally exposes `assistant.threads.setStatus` — a purpose-built animated status API for AI agents — which pairs with the existing emoji reactions.

## What Changes

- **All interfaces** (`ChannelAdapter` trait + `ChannelRunner`): add a new `on_message_received` lifecycle hook called immediately on arrival (before the per-conversation lock is acquired). Adapters use this to add an ⏳ hourglass reaction. Existing `on_turn_start` is responsible for removing the hourglass when processing begins.
- **Slack**: call `assistant.threads.setStatus` (with animated loading messages) in `on_turn_start`, alongside the existing 👀 emoji reaction. Keep all existing reactions.
- **Matrix**: send `PUT /_matrix/client/v3/rooms/{roomId}/typing/{userId}` in `on_turn_start`; clear in `on_turn_success`/`on_turn_error`. Add ⏳ reaction on `on_message_received`, clear on `on_turn_start`.
- **Mattermost**: `POST /api/v4/users/me/typing` in `on_turn_start` (server auto-expires). Add ⏳ reaction on `on_message_received`.
- **Nextcloud Talk**: `POST /ocs/v2.php/apps/spreed/api/v1/chat/{token}/typing` in `on_turn_start`/`on_turn_success`/`on_turn_error`. Add ⏳ reaction on `on_message_received`.
- **Signal** (via signal-cli-rest-api): no native typing or reaction API — no change beyond the trait no-op.

## Capabilities

### New Capabilities

- `interface-queue-signal`: `on_message_received` hook on `ChannelAdapter` + `ChannelRunner` call site; each adapter adds ⏳ on receipt and clears it in `on_turn_start`
- `slack-agent-status`: call `assistant.threads.setStatus` in Slack `on_turn_start` (alongside existing 👀 reaction)
- `matrix-typing-indicator`: send and clear Matrix typing events (`m.typing`) during agent processing turns
- `mattermost-typing-indicator`: broadcast Mattermost `user_typing` via `POST /api/v4/users/me/typing` in `on_turn_start`
- `nextcloud-typing-indicator`: send Nextcloud Talk typing notifications during agent processing turns (best-effort)

### Modified Capabilities

## Impact

- `crates/core/src/channel.rs` — new `on_message_received` default-no-op method on `ChannelAdapter` trait
- `crates/runtime/src/channel_runner.rs` — call `on_message_received` before `tokio::spawn`
- `crates/interface-slack/src/` — `client.rs` (new `set_agent_status` method), `adapter.rs` (`on_message_received`, updated `on_turn_start`)
- `crates/interface-matrix/src/` — `client.rs` (new `send_typing`, `add_reaction`, `remove_reaction`), `adapter.rs` (all hooks)
- `crates/interface-mattermost/src/` — `client.rs` (new `send_typing`, reaction methods), `adapter.rs` (all hooks)
- `crates/interface-nextcloud/src/` — `adapter.rs` (all hooks)
- No new external dependencies; all use existing `reqwest` HTTP clients
- No config, API schema, or database changes
