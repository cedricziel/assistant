## Context

The assistant has multiple chat interface adapters (Slack, Matrix, Mattermost, Nextcloud, Signal). All share a common dispatch loop in `ChannelRunner` (`crates/runtime/src/channel_runner.rs`).

When a message arrives, `ChannelRunner` serialises turns per conversation using a `HashMap<Uuid, Arc<Mutex<()>>>`. The flow is:

```
message arrives
  → on_message_received  (NEW — before spawn)
  → tokio::spawn
      → lock.lock().await   (waits if another turn is in flight)
      → dispatch()
          → on_turn_start   (processing begins)
          → orchestrator.run_turn_with_tools()
          → on_turn_success / on_turn_error
```

This means:

- There is already a "turn start" seam (`on_turn_start`).
- There is currently **no** "message received" seam before the lock wait.
- Slack already uses `on_turn_start` to add a 👀 reaction and `on_turn_success` for ✅.
- Matrix, Mattermost, and Nextcloud `on_turn_start` are currently stubs.
- Slack exposes `assistant.threads.setStatus` — a dedicated animated loading-state API for AI agents.
- Matrix, Mattermost, and Nextcloud have native typing-indicator endpoints.

## Goals / Non-Goals

**Goals:**

- Add `on_message_received` hook to `ChannelAdapter` (core trait + `ChannelRunner` callsite) so adapters can signal "queued" immediately upon receipt
- Each adapter adds ⏳ hourglass reaction on `on_message_received` and removes it in `on_turn_start`
- Slack calls `assistant.threads.setStatus` in `on_turn_start` (alongside existing 👀 reaction)
- Matrix/Mattermost/Nextcloud send native typing indicators in `on_turn_start`, clear on end
- All signals are best-effort — failures must not propagate

**Non-Goals:**

- Streaming/partial output during inference
- Keep-alive pings for turns > 30 s (Matrix timeout)
- Signal interface (no API available)
- Removing or replacing existing emoji reactions

## Decisions

### D1: New `on_message_received(msg: &ChannelMessage)` hook on `ChannelAdapter`

Called in `ChannelRunner::run()` immediately after resolving `conv_id`, before `tokio::spawn`. The `ChannelMessage` is still owned at this point so no clone is needed for the hook call; the existing clone for spawn proceeds unchanged.

Default: no-op. Adapters override to add ⏳.

**Alternative**: pass just `(user, platform_message_id)` instead of the full `ChannelMessage`. Rejected — giving adapters the full message is more flexible and consistent with how `platform_tools()` works.

### D2: `on_turn_start` is responsible for clearing the hourglass

The hourglass is removed in `on_turn_start`, not in a separate hook. This keeps the contract simple: receipt adds ⏳, processing-start clears it and adds 👀/typing. No new "on_dequeue" hook needed.

**Alternative**: dedicated `on_turn_dequeue` hook. Rejected — unnecessary indirection for one emoji removal.

### D3: Slack keeps both emoji reactions AND `assistant.threads.setStatus`

`setStatus` provides the prominent animated UI in Slack's AI assistant surface. Emoji reactions (👀 on start, ✅ on success) remain for non-assistant-thread contexts (regular channel messages).

`setStatus` is called in `on_turn_start` with a rotating `loading_messages` array. It auto-clears when the bot posts a reply. No explicit clear needed.

**Alternative**: replace emoji reactions with `setStatus` only. Rejected per user preference ("keep the emoji").

### D4: Matrix hourglass via `m.reaction` event, typing via dedicated endpoint

Matrix reactions use `PUT /rooms/{roomId}/send/m.reaction/{txnId}`. To remove, we must `PUT /rooms/{roomId}/redact/{eventId}/{txnId}`. The adapter needs to store the reaction event ID to redact it later.

Typing uses `PUT /_matrix/client/v3/rooms/{roomId}/typing/{userId}` with 30 s timeout; explicit clear on turn end.

### D5: Mattermost hourglass via emoji reaction, typing fire-and-forget

Mattermost emoji reactions: `POST /api/v4/reactions` (add), `DELETE /api/v4/users/me/posts/{postId}/reactions/{emojiName}` (remove). The message `post_id` is available in the inbound message.

Typing: `POST /api/v4/users/me/typing` with `channel_id`. Auto-expires server-side — no explicit clear.

### D6: Nextcloud hourglass via reaction, typing via Talk API

Nextcloud Talk reactions: `POST /ocs/v2.php/apps/spreed/api/v1/reaction/{token}/{messageId}` (add), `DELETE` (remove). Requires `OCS-APIRequest: true`.

Typing: `POST /ocs/v2.php/apps/spreed/api/v1/chat/{token}/typing` with `{ "typing": true/false }`. Gracefully handles 404 on Talk < 17.

### D7: All failures swallowed at `debug!` level

No typing or reaction signal failure should propagate to the caller. All calls use `let _ = ...` or log at `debug!`.

## Risks / Trade-offs

- [Matrix event ID for redact must be stored in adapter state] → The adapter holds a `DashMap<platform_id, event_id>` or `Mutex<HashMap>` for in-flight redact targets. Memory is bounded by concurrent conversations.
- [Matrix rate-limiting] → errors swallowed (D7).
- [Nextcloud Talk < 17 returns 404 for typing] → errors swallowed (D7).
- [Matrix turns > 30 s show indicator cleared mid-turn] → Accepted; keep-alive is a future change.
- [Mattermost `on_message_received` before spawn: adapter must not block] → all reaction calls are async and fire-and-forget with `tokio::spawn` internally, keeping the main loop unblocked.

## Migration Plan

No config, schema, or database changes. Deploy updated binary. The new `on_message_received` default is a no-op, so adapters without overrides are unaffected.

## Open Questions

- Should ⏳ be removed before adding 👀 (sequential) or in parallel? Current design: remove ⏳ first, then add 👀 in `on_turn_start`, synchronously.
- Nextcloud Talk version detection: current design accepts 404 silently.
