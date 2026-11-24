## Context

When a user records and sends a voice message, the flow in `chat_provider.dart` `_streamVoiceMessage()` works as follows:

1. A user `ChatMessage` is created with placeholder text `"🎤 Voice message"` and `status: sending` (line 848)
2. Audio bytes are uploaded to the server via `api.sendVoiceMessage(conversationId, audioBytes, mimeType)` (line 870)
3. When `TranscriptEvent` arrives (line 878), the user message content is **replaced** with the transcript text and the audio bytes are discarded
4. The original recorded audio is never stored on the `ChatMessage` — it only exists as a local variable during `_streamVoiceMessage()`

The `AudioPlayerWidget` already exists for assistant TTS playback but uses lazy-loaded bytes from a server endpoint. For user voice messages, the bytes are available locally — no server fetch needed.

## Goals / Non-Goals

**Goals:**

- Render user voice messages as a mini inline audio player with play/pause and progress
- Show the transcript collapsed by default below the player, expandable on tap
- Preserve audio bytes on the `ChatMessage` so they survive state updates during streaming
- Reuse or adapt the existing `AudioPlayerWidget` for the inline player

**Non-Goals:**

- Waveform visualisation (out of scope — use a simple linear progress bar)
- Persisting audio bytes across app restarts (bytes are in-memory only; on reload, show transcript-only)
- Changing the server API or adding audio download endpoints for user messages

## Decisions

### D1: Add `audioBytes` and `audioMimeType` fields to `ChatMessage`

**Choice:** Add two nullable fields to `ChatMessage`:

```dart
final Uint8List? audioBytes;
final String? audioMimeType;
```

**Why:** The audio bytes are available as a parameter to `_streamVoiceMessage()`. Storing them on the message is the simplest way to make them available to the rendering layer. The fields are final (set once at creation, preserved through `copyWith`).

**Alternative considered:** Store audio in a separate `Map<String, Uint8List>` provider keyed by message ID. Rejected — adds indirection and a second state to keep in sync. The bytes belong to the message.

### D2: Populate audio on the initial user message, not on TranscriptEvent

**Choice:** Set `audioBytes` and `audioMimeType` on the user `ChatMessage` when it's first created (line 848), before streaming starts. The `TranscriptEvent` handler updates `content` but leaves audio fields unchanged via `copyWith`.

**Why:** The audio bytes are available at message creation time. Setting them early means they're never at risk of being lost during state transitions.

### D3: Render as mini player + collapsible transcript

**Choice:** When `message.isUser && message.audioBytes != null`, render:

1. A compact audio player row (play/pause icon button + linear progress bar + duration label)
2. Below it, a collapsed transcript row: chevron + first ~40 chars of transcript, tap to expand full text

**Why:** Matches the user's stated preference. The audio is primary (it's what they recorded), the transcript is secondary (machine-generated, may contain errors).

**Layout:**

```
┌────────────────────────────────────────┐
│  ▶  ━━━━━━━━●━━━━━━━━━━━  0:04        │
│  ▸ "What's the weather in Berlin t..." │
└────────────────────────────────────────┘
```

### D4: Reuse `audioplayers` package with `BytesSource`

**Choice:** Use the same `audioplayers` package and `BytesSource(audioBytes)` pattern as `AudioPlayerWidget`. Extract shared playback logic if the overlap is significant, otherwise keep a separate `_VoiceMessagePlayer` widget to avoid over-abstracting.

**Why:** `audioplayers` is already a dependency. `BytesSource` works well for in-memory audio. No additional packages needed.

### D5: On conversation reload, fall back to transcript-only

**Choice:** When loading from history (`loadConversation`), user voice messages won't have `audioBytes` (the server doesn't store raw audio for download). These messages render as plain transcript text — same as today.

**Why:** Adding a server-side audio download endpoint is out of scope. The audio player is a live-session enhancement. Users who reload will see the transcript, which is the permanent record.

## Risks / Trade-offs

- **Memory usage:** Storing audio bytes (~50-200KB per message) in the widget tree. For a typical session with a few voice messages this is negligible. If a user sends dozens of long voice messages, memory could grow. Mitigation: audio is `Uint8List` (compact), and messages are disposed when navigating away from the conversation.
- **`copyWith` overhead:** `Uint8List` is copied by reference in `copyWith`, not cloned. This is correct — the bytes are immutable after recording.
- **MIME type compatibility:** `audioplayers` `BytesSource` handles MP3, AAC, Opus, and WAV. The recorder outputs one of these formats (platform-dependent). No conversion needed.

## Migration Plan

No migration. New fields default to `null`. Existing messages and stored history are unaffected.
