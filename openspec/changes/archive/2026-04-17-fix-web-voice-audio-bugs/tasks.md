## 1. Server — expose message UUID in TurnResult

- [x] 1.1 Add `pub message_id: Option<Uuid>` to `TurnResult` in `crates/runtime/src/orchestrator/mod.rs`
- [x] 1.2 Populate `message_id` in the orchestrator turn loop where the assistant message is persisted to the DB (set it from the returned row ID)
- [x] 1.3 Confirm all existing callers of `TurnResult` (CLI, Slack, Mattermost, Matrix, Signal, Nextcloud interfaces) compile cleanly with the new optional field — no behaviour change needed there

## 2. Server — emit message_id in done SSE event

- [x] 2.1 In `send_message` handler (`crates/web-ui/src/api/mod.rs`): update the `done_data` JSON to include `"message_id"` from `turn_result_rx` result when `TurnResult.message_id` is `Some`
- [x] 2.2 In `send_voice_message` handler (same file): same change — include `"message_id"` in the `done_data` JSON
- [x] 2.3 Add/update unit tests in `api/mod.rs` that assert the `done` event body contains a `message_id` field after a successful turn

## 3. Flutter — TranscriptEvent

- [x] 3.1 Add `class TranscriptEvent extends StreamEvent` with a `final String transcript` field to `app/lib/api/models/stream_event.dart`
- [x] 3.2 Add a `TranscriptEvent.fromJson` factory that reads `content` from the JSON payload
- [x] 3.3 In `parseSseByteStream` (`app/lib/api/api_client.dart`): add an `else if (eventType == 'transcript')` branch that yields `TranscriptEvent.fromJson(json)`
- [x] 3.4 Add unit tests in `app/test/unit/api/` covering: transcript frame is parsed, unknown event type is ignored

## 4. Flutter — DoneEvent gains optional messageId

- [x] 4.1 Add `final String? messageId` to `DoneEvent` in `stream_event.dart`
- [x] 4.2 Update `DoneEvent.fromJson` to parse `message_id` from the JSON payload (null if absent)
- [x] 4.3 Add unit test: done event with `message_id` field parses correctly; done event without it yields `messageId == null`

## 5. Flutter — fix \_streamVoiceMessage in chat_provider.dart

- [x] 5.1 Add a `TranscriptEvent` handler in `_streamVoiceMessage`: when received, update the user message bubble content to `event.transcript` and set `status: MessageStatus.ok`
- [x] 5.2 Remove the incorrect user-bubble overwrite from the `DoneEvent` handler in `_streamVoiceMessage` (the block that sets content to `'[Voice] ${event.content}'`)
- [x] 5.3 In the `DoneEvent` handler of `_streamVoiceMessage`: use `event.messageId` (when non-null) as the ID for the finalized assistant `ChatMessage` instead of `'assistant-${DateTime.now().millisecondsSinceEpoch}'`

## 6. Flutter — fix \_streamMessage in chat_provider.dart

- [x] 6.1 In the `DoneEvent` handler of `_streamMessage`: use `event.messageId` (when non-null) as the ID for the finalized assistant `ChatMessage` instead of `'assistant-${DateTime.now().millisecondsSinceEpoch}'`

## 7. Verification

- [x] 7.1 Run `cargo test -p assistant-web-ui` — all existing and new server tests pass
- [x] 7.2 Run `flutter test` in `app/` — all existing and new Dart tests pass
- [ ] 7.3 Manual end-to-end: record a voice message in the web UI — user bubble shows transcribed text, assistant bubble shows agent reply
- [ ] 7.4 Manual end-to-end: tap play button on a freshly received assistant message — audio plays without a 400 error
- [x] 7.5 Run `make lint && make format` — no warnings or formatting changes
