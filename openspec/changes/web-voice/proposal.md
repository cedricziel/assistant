## Why

The web UI currently supports only text-based interaction. Users on other interfaces (Slack, Signal) can already send voice messages that are automatically transcribed and injected into the conversation. The web UI should support the same — and go further by letting the assistant respond with synthesized speech, enabling a full voice conversation loop.

## What Changes

- Add `TtsProvider` trait and implementations (OpenAI, Deepgram) to `assistant-transcription`
- Add `TtsConfig` (`[tts]` section) to core config types
- Wire STT and TTS providers into the web UI server (`ApiState`)
- New endpoint: `POST /api/conversations/{id}/voice` — accepts recorded audio, transcribes, sends as message (SSE stream)
- New endpoint: `GET /api/messages/{msg_id}/audio` — on-demand TTS synthesis for any assistant message
- New endpoint: `GET /api/audio/{audio_id}` — serve pre-synthesized audio produced by the `voice_response` tool
- New builtin tool: `voice_response(text, voice?)` — assistant can proactively synthesize a voiced reply
- Flutter: microphone button in the chat input row (record → upload → auto-transcribe)
- Flutter: playable audio bubble on assistant messages (tap ▶ to synthesize and play)
- Flutter: auto-play when the assistant uses the `voice_response` tool

## Capabilities

### New Capabilities

- `voice-send`: Record and send voice messages from the web UI; audio is transcribed server-side and injected into the conversation as a text message.
- `voice-receive`: Assistant messages can be played back as synthesized speech via an on-demand TTS endpoint; the assistant may also proactively voice a reply using the `voice_response` tool.

### Modified Capabilities

_(none — existing text-chat capability is unchanged)_

## Impact

**Rust crates:**

- `assistant-transcription`: new `TtsProvider` trait, `TtsRequest`/`TtsResult` types, OpenAI and Deepgram TTS implementations, `build_tts_provider()` helper
- `assistant-core`: new `TtsConfig` struct, `[tts]` section in `AssistantConfig`
- `assistant-tool-executor`: new `VoiceResponseHandler` builtin tool
- `assistant-web-ui`: `ApiState` gains `tts_provider` and `audio_store` fields; new axum routes; new `AudioStore` (in-memory, TTL-managed); SSE stream gains `audio_ready` event type

**Flutter app:**

- `pubspec.yaml`: add `record` and `audioplayers` packages
- `app/lib/features/chat/chat_screen.dart`: mic button, `VoiceRecorder` widget, audio-player bubble
- `app/lib/api/client.dart` / endpoint wrappers: `sendVoiceMessage()`, `fetchMessageAudio()`, `fetchAudio()`

**Config:**

- New `[tts]` section in `~/.assistant/config.toml`; initially shares credentials with `[transcription]` when using OpenAI/Deepgram

**Dependencies (no new Rust crates required):** OpenAI TTS uses the existing `reqwest`-based HTTP client already in `assistant-transcription`.
