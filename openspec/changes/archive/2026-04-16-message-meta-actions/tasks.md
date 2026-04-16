# Tasks: Message Meta Actions

## Tasks

### 1. Fix Deepgram TTS auth header

- [x] Change `.bearer_auth()` to `.header("Authorization", format!("Token {}", self.api_key))` in `crates/transcription/src/deepgram_tts.rs:67`
- [x] Update wiremock test to assert `Token` auth header format
- [x] Run `cargo test -p assistant-transcription`

### 2. Build `_MetaActionRow` widget

- [x] Create `_MetaActionRow` stateless widget in `chat_screen.dart`
- [x] Create `_MetaActionButton` base widget (icon + label, muted styling)
- [x] Implement `_CopyAction` — copies `message.content` to clipboard
- [x] Implement visibility rules: alignment follows bubble side, conditional action list
- [x] Wire into `_MessageBubble` — render below the bubble `Container`

### 3. Implement `_ReadAloudAction` with state machine

- [x] Create stateful `_ReadAloudAction` widget with `ReadAloudState` enum (idle/loading/playing/error)
- [x] Manage `AudioPlayer` lifecycle (init, dispose, onComplete listener)
- [x] Idle: `🔊 Read aloud` — tap triggers fetch
- [x] Loading: `⟳ Loading…` — spinner while awaiting TTS
- [x] Playing: `■ Stop reading` — tap stops playback
- [x] Error: show inline warning below action row, auto-dismiss after 4s
- [x] Only render when `!message.isUser && capabilities.voiceReceive && message.audioId == null`

### 4. Consolidate retry button into meta row

- [x] Add `_RetryAction` to meta row (failed messages only)
- [x] Remove existing standalone retry button from `_MessageBubble` (lines ~668-688)
- [x] Preserve same `onRetry` callback behavior

### 5. Add stop action for streaming

- [x] Add `_StopAction` to meta row when `message.isStreaming`
- [x] Wire to existing stream cancellation mechanism

### 6. Update inline audio player condition

- [x] Change audio player guard from `capabilities.voiceReceive && message.ttsAvailable && !message.isStreaming` to `message.audioId != null && !message.isStreaming`
- [x] Keep `AudioPlayerWidget` for real audio (inline, inside bubble)
- [x] Remove on-demand TTS fallback from `fetchMessageAudio` in `chat_screen.dart` (the meta action handles this now)

### 7. Clean up ttsAvailable streaming logic

- [x] In `chat_provider.dart` `DoneEvent` handler: remove `(caps.voiceReceive && event.content.isNotEmpty)` fallback
- [x] Keep `ttsAvailable = true` only on `AudioReadyEvent`
- [x] Both stream handler locations (~lines 935-943 and 1095-1103)

### 8. Write tests

- [x] Widget test: meta row renders `Copy` for all messages
- [x] Widget test: meta row renders `Read aloud` only for assistant + voiceReceive + no audioId
- [x] Widget test: meta row renders `Retry` only for failed messages
- [x] Widget test: `_ReadAloudAction` state transitions
- [x] Widget test: error state auto-dismisses
- [x] Widget test: inline audio player only when audioId present
- [x] Rust test: Deepgram TTS `Token` auth header

## Dependencies

```
Task 1 (auth fix) ── independent, can ship alone
Task 2 (meta row) ── foundation for 3, 4, 5
Task 3 (read aloud) ── depends on 2
Task 4 (retry consolidation) ── depends on 2
Task 5 (stop action) ── depends on 2
Task 6 (inline audio guard) ── depends on 3
Task 7 (ttsAvailable cleanup) ── depends on 6
Task 8 (tests) ── depends on all above
```

Task 1 can be done first as a standalone fix. Tasks 2-7 form the UI change.
