# Proposal: Message Meta Actions & Audio Button Fix

## Status: proposed

## Problem

The chat UI has three related issues:

1. **Audio play button appears on every assistant message** when TTS is configured, regardless of whether audio actually exists. This is noisy and misleading.
2. **TTS silently fails** — the Deepgram TTS provider uses `Bearer` auth instead of `Token` auth, causing 401s. Even when fixed, the widget swallows errors with no user feedback.
3. **No meta action row** — copy, retry, and read-aloud are either missing or inconsistently placed (retry is a standalone button below the bubble).

## Proposed Solution

Introduce a **meta action row** below each message bubble that consolidates contextual actions. Replace the current in-bubble audio play button with this system.

### Meta actions by message state

| State               | Actions available               |
| ------------------- | ------------------------------- |
| Assistant message   | `Read aloud`\*, `Copy`          |
| Assistant failed    | `Read aloud`\*, `Copy`, `Retry` |
| Assistant streaming | `Stop`                          |
| User message        | `Copy`                          |
| User failed         | `Copy`, `Retry`                 |

\* Only when `voiceReceive` capability is `true`.

### Audio distinction

- **Real audio** (agent replied with voice via `AudioReadyEvent`): inline player inside the bubble with progress bar. No "Read aloud" action needed.
- **On-demand TTS** (no audio exists, but server can synthesize): "Read aloud" in meta action row. States: Idle → Loading → Playing ("Stop reading") → Idle. Error shown inline below the action row, fades after a few seconds.

### Visual treatment

- Font: 12px, muted color (`onSurface` at 60% opacity)
- No background or border — just icon + label in a horizontal row
- 16px spacing between actions
- Alignment follows bubble (left for assistant, right for user)
- Always visible (no hover-to-reveal — works better with touch)

### Bug fix (auth)

`deepgram_tts.rs:67` uses `.bearer_auth()` but Deepgram requires `Token` auth (as the STT provider correctly does). One-line fix.

## Scope

### In scope

- Meta action row component (`_MetaActionRow` widget)
- Copy action (all messages)
- Read aloud action with state machine (idle/loading/playing/error)
- Consolidate existing retry button into meta row
- Stop action during streaming
- Fix Deepgram TTS auth header
- Remove old `AudioPlayerWidget` from bubble interior (for on-demand TTS)
- Keep inline audio player for messages with real `audioId`

### Out of scope

- Audio progress bar for TTS playback (simple play/stop is enough)
- Persisting TTS audio server-side
- Any changes to voice recording input
- Mobile/iOS app (not yet built)

## Affected code

### Rust

- `crates/transcription/src/deepgram_tts.rs` — auth header fix

### Flutter

- `app/lib/features/chat/chat_screen.dart` — bubble layout, meta action row, remove old audio button placement
- `app/lib/features/chat/audio_player_widget.dart` — keep for inline real-audio player, or refactor into meta action
- New: meta action row widget (likely in `chat_screen.dart` or extracted)

## Risks

- Meta row adds vertical space to every message. Mitigated by small font and tight padding.
- "Read aloud" TTS latency may feel slow for long messages. Mitigated by loading state with spinner.
