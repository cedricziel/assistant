## Why

Two regressions shipped with the `web-voice` change: the user's voice bubble displays the agent's reply text instead of the transcription, and tapping the play button on a freshly received assistant message returns a 400 from the server. Both bugs make the voice feature unusable end-to-end.

## What Changes

- Add `TranscriptEvent` sealed-class variant to `stream_event.dart` and parse `event: transcript` frames in `_parseSse` (currently silently discarded).
- Handle `TranscriptEvent` in `_streamVoiceMessage`: update the user message bubble with the actual spoken text when received.
- Remove the incorrect `DoneEvent` user-bubble overwrite in `_streamVoiceMessage` that copies the assistant's reply text into the user bubble prefixed with `[Voice]`.
- **Server**: include the saved DB message UUID in the `done` SSE event payload for both `send_message` and `send_voice_message` handlers (`crates/web-ui/src/api/mod.rs`).
- **Client**: extend `DoneEvent` to parse `message_id` from the `done` payload and use it as `ChatMessage.id`, replacing the synthetic `assistant-<timestamp>` string so `GET /api/messages/{id}/audio` receives a valid UUID.

## Capabilities

### New Capabilities

- `voice-transcript-event`: Client-side handling of the `transcript` SSE event emitted by the voice endpoint, so the spoken text appears correctly in the user bubble.

### Modified Capabilities

- `voice-send`: The `done` SSE event payload gains a `message_id` field (real DB UUID); client uses it to set `ChatMessage.id`. Server-side change to `send_message` and `send_voice_message` handlers.

## Impact

- `app/lib/api/models/stream_event.dart` — new `TranscriptEvent` class, `DoneEvent` gains optional `messageId` field.
- `app/lib/api/api_client.dart` — `_parseSse` parses `transcript` events.
- `app/lib/features/chat/chat_provider.dart` — `_streamVoiceMessage` handles `TranscriptEvent`; `DoneEvent` handler corrected.
- `crates/web-ui/src/api/mod.rs` — `done` SSE event in both message handlers emits `message_id`.
- No DB schema changes. No new dependencies. No breaking API changes (additive field on `done` event).
