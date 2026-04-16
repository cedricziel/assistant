## Context

The `web-voice` change shipped STT (voice recording → transcription → agent response) and TTS (play button on assistant messages) for the web UI. Two bugs survived into production:

1. **Wrong bubble content**: The server emits `event: transcript` immediately after receiving the audio, carrying the user's spoken text. The Flutter SSE parser has no handler for this event type and discards it silently. At stream end, `_streamVoiceMessage` tries to fix the user bubble by writing `[Voice] ${event.content}` — but `DoneEvent.content` is the _assistant's_ reply, so the agent's words end up in the user bubble.

2. **Invalid message ID on play**: After streaming, `ChatMessage.id` is set to `'assistant-<milliseconds>'`. `GET /api/messages/{id}/audio` calls `Uuid::parse_str` on that string, fails, and returns 400. The fallback path only works for messages loaded from history (which carry real DB UUIDs). Freshly streamed messages have no real UUID stored client-side.

Both fixes are small and confined: one adds a missing SSE event type; the other threads the real DB UUID through the `done` event.

## Goals / Non-Goals

**Goals:**

- User voice bubble shows the transcribed spoken text (from `transcript` SSE event).
- On-demand TTS play button works for messages received in the current session.
- No regressions to text-only chat or history-loaded messages.

**Non-Goals:**

- Changes to transcription provider, TTS provider, or audio store.
- Streaming TTS or pre-synthesis.
- OpenAPI schema changes for SSE event payloads (SSE bodies are not modelled by openapi-generator).

## Decisions

### D1: Surface transcript via a new `TranscriptEvent` — not via `DoneEvent`

**Decision:** Add `TranscriptEvent(String transcript)` to the sealed `StreamEvent` hierarchy. Parse `event: transcript` frames in `_parseSse`. In `_streamVoiceMessage`, handle `TranscriptEvent` to update the user bubble. Remove the broken `DoneEvent` user-bubble overwrite.

**Rationale:** The server already emits `event: transcript` before any token events. Reusing `DoneEvent` for the transcript would require the server to duplicate the text in two events, or clients to guess which field to use. A dedicated event type is unambiguous and consistent with the existing `AudioReadyEvent` pattern.

**Alternative considered:** Overload the existing `DoneEvent` with a `transcript` field. Rejected — `DoneEvent` already has a clear contract (`role + content = final assistant reply`); adding a user-transcript field to an assistant-role event is confusing and breaks the single-responsibility of that event.

### D2: Add `message_id` to the `done` SSE event payload

**Decision:** Both `send_message` and `send_voice_message` in `api/mod.rs` emit the final `done` event as:

```json
{ "role": "assistant", "content": "<text>", "message_id": "<uuid>" }
```

where `message_id` is taken from `TurnResult`. The Flutter `DoneEvent` gains an optional `String? messageId` field. `_streamMessage` and `_streamVoiceMessage` use it to set `ChatMessage.id` when present; fall back to the timestamp ID otherwise.

**Rationale:** The server already knows the UUID (it is returned by `submit_turn` → `TurnResult`). Emitting it in `done` is zero-cost. The client then has a real UUID for `fetchMessageAudio` without any extra round-trip. The field is optional so old server / new client and new server / old client both work gracefully.

**Alternative considered:** Re-fetch the conversation after `done` to get real UUIDs. Rejected — adds latency, a second HTTP call, and is already done (for title refresh) via `conversationListProvider.refresh()` which does not update `ChatMessage.id` objects in the local state.

**Alternative considered:** Only show the play button once the conversation is reloaded. Rejected — poor UX; the play button appears immediately after streaming ends.

### D3: `TurnResult` already contains the message ID — verify before relying on it

**Decision:** Check `TurnResult` in `assistant-runtime` to confirm it exposes the saved message UUID. If not, add the field.

**Rationale:** `submit_turn` is the natural place to return the persisted message ID. Adding it to `TurnResult` is the minimal change; no new return channel is needed.

## Risks / Trade-offs

- **`TurnResult` may not expose message ID today** → Inspect `crates/runtime`; add field if missing. Low risk — it's a simple struct addition with no downstream callers that need to change.
- **Old server + new client**: `messageId` is null in `DoneEvent`; client falls back to timestamp ID. Play button still broken on old server, but no regression vs. today.
- **New server + old client**: extra `message_id` key in `done` JSON is ignored by old client. No breakage.
- **`transcript` event ordering**: server emits transcript _before_ starting the orchestrator turn. If the SSE connection drops between transcript and done, the user bubble will still show the transcript, which is correct and preferable to showing nothing.

## Migration Plan

1. Land server change (`message_id` in `done`) — backward-compatible, deploy any time.
2. Land Flutter client changes — `TranscriptEvent`, `DoneEvent.messageId`, provider fixes.
3. No DB migrations, no config changes, no feature flags required.
