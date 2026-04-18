## Why

When a user sends a voice message, the recorded audio is uploaded and the user bubble is completely replaced with the transcript text. The original audio is discarded and cannot be replayed. Users expect to see their voice message rendered as an audio player (like WhatsApp, Telegram, iMessage) with the transcript available but secondary.

## What Changes

- Preserve recorded audio bytes and MIME type on the user `ChatMessage` after sending
- Render user voice messages as a mini audio player widget (play/pause, waveform/progress bar, duration)
- Show the transcript collapsed by default below the player, expandable on tap
- Keep the transcript as the message `content` for search and history purposes

## Capabilities

### New Capabilities

- `voice-message-playback`: User voice messages render as mini audio players with play/pause and progress
- `voice-message-transcript-toggle`: Transcript text is collapsed by default, expandable on tap

### Modified Capabilities

- `chat-voice-send`: Voice message flow preserves audio bytes on the `ChatMessage` instead of discarding them

## Impact

- `app/lib/features/chat/chat_provider.dart` — Add `audioBytes` and `audioMimeType` fields to `ChatMessage`; preserve audio in `_streamVoiceMessage()` when `TranscriptEvent` arrives
- `app/lib/features/chat/chat_screen.dart` — Render user bubbles with `audioBytes != null` as mini audio player + collapsible transcript instead of plain text
- New widget or reuse of `AudioPlayerWidget` adapted for inline user message display
- No backend changes (audio bytes are already available client-side during the send flow)
- No API changes
