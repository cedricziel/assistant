# Design: Message Meta Actions

## Architecture

### Meta Action Row Widget

A new `_MetaActionRow` stateless widget rendered below every `_MessageBubble`. It receives the message state and capabilities, then conditionally renders action buttons.

```
┌──────────────────────────────────────┐
│         Message Bubble               │
└──────────────────────────────────────┘
 ┌─ _MetaActionRow ───────────────────┐
 │  [action] [action] [action]        │
 └────────────────────────────────────┘
```

### Component Hierarchy

```
_MessageBubble (existing, modified)
├── Container (bubble)
│   ├── message content / markdown
│   └── inline audio player (only when audioId != null)
│
└── _MetaActionRow (new)
    ├── _ReadAloudAction (stateful — manages TTS lifecycle)
    ├── _CopyAction (stateless)
    ├── _RetryAction (stateless, moved from current location)
    └── _StopAction (stateless, for streaming)
```

### Read Aloud State Machine

```
         tap
  IDLE ──────► LOADING
   ▲              │
   │              │ audio received
   │              ▼
   │          PLAYING
   │           │   │
   │  done     │   │ tap
   └───────────┘   │
   ▲               │
   │    stop       │
   └───────────────┘

  ERROR: shown as inline warning below action row.
         Resets to IDLE after ~4 seconds or next tap.
```

```dart
enum ReadAloudState { idle, loading, playing, error }
```

The `_ReadAloudAction` widget is stateful and owns an `AudioPlayer` instance (from `audioplayers` package, already a dependency). It calls `fetchMessageAudio` lazily on first tap, caches the bytes, and manages playback.

### Action Visibility Rules

```dart
// Pseudo-code for _MetaActionRow.build()

actions = [];

if (message.isStreaming) {
  actions.add(StopAction(onStop));
} else {
  // Read aloud: assistant messages only, TTS available, no real audio
  if (!message.isUser && capabilities.voiceReceive && message.audioId == null) {
    actions.add(ReadAloudAction(fetchAudio));
  }

  // Copy: all messages with content
  if (message.content.isNotEmpty) {
    actions.add(CopyAction(message.content));
  }

  // Retry: failed messages only
  if (message.status == MessageStatus.failed && onRetry != null) {
    actions.add(RetryAction(onRetry));
  }
}
```

### Inline Audio Player (real audio)

The existing `AudioPlayerWidget` stays but is only rendered when `message.audioId != null`. It remains **inside** the bubble as it represents content the agent intentionally produced.

```dart
// Inside _MessageBubble.build(), assistant branch:
if (message.audioId != null && !message.isStreaming)
  AudioPlayerWidget(fetchAudio: fetchMessageAudio),
```

The current condition `capabilities.voiceReceive && message.ttsAvailable && !message.isStreaming` is replaced.

### ttsAvailable Semantics Change

Currently `ttsAvailable` is set `true` for every non-empty assistant message when TTS is configured. With this change:

- **Server side**: `tts_available` in `MessageSummary` can stay as-is (indicates TTS _could_ work). The meta action row uses `capabilities.voiceReceive` directly.
- **Client side**: The `DoneEvent` handler no longer needs to set `ttsAvailable` based on `caps.voiceReceive && event.content.isNotEmpty`. It only sets `ttsAvailable = true` on `AudioReadyEvent`.
- The `ttsAvailable` field effectively becomes "has real audio" and could be renamed to `hasAudio` in a follow-up, but that's not required now.

### Deepgram TTS Auth Fix

```rust
// crates/transcription/src/deepgram_tts.rs — line 67
// Before:
.bearer_auth(&self.api_key)

// After:
.header("Authorization", format!("Token {}", self.api_key))
```

This matches the STT provider's auth pattern in `deepgram.rs:205`.

### Styling

```dart
class _MetaActionButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;
  final Color? color; // override for error/danger states

  // Renders as:
  // - Row(icon, SizedBox(4), Text(label))
  // - Icon size: 14
  // - Text: 12px, colorScheme.onSurface.withOpacity(0.6)
  // - InkWell with small splash radius
  // - No padding/margin beyond natural text bounds
}
```

Action row padding: `EdgeInsets.only(top: 4)`, horizontal spacing between actions: `SizedBox(width: 16)`.

Alignment: `CrossAxisAlignment.start` for assistant, `CrossAxisAlignment.end` for user — matching the bubble alignment.

### Error Display

When TTS fails, a small warning appears below the action row:

```dart
// Inside _ReadAloudAction, when state == error:
Padding(
  padding: EdgeInsets.only(top: 2),
  child: Text(
    'Could not generate audio',
    style: TextStyle(fontSize: 11, color: colorScheme.error),
  ),
)
```

Auto-dismiss after 4 seconds via a `Timer` that resets state to `idle`.

### Consolidation of Existing Retry Button

The current retry button in `_MessageBubble` (lines 668-688 of `chat_screen.dart`) is removed and replaced by `_RetryAction` in the meta row. Same `onRetry` callback, just rendered in the new location.

## Migration

This is a UI-only change with one Rust bug fix. No database migrations, no API changes, no protocol changes.

## Testing

- Widget test: `_MetaActionRow` renders correct actions per message state
- Widget test: `_ReadAloudAction` state transitions (idle → loading → playing → idle)
- Widget test: error state shows warning and auto-dismisses
- Widget test: retry action appears only on failed messages
- Widget test: copy action copies content to clipboard
- Widget test: inline audio player only appears when `audioId != null`
- Unit test (Rust): Deepgram TTS sends `Token` auth header (update existing wiremock test)
