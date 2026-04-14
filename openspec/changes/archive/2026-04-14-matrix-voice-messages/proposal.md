## Why

The Matrix interface only handles `m.room.message` text events and silently drops `m.audio` voice messages and `m.image` image messages entirely. Users who send voice or images in Matrix rooms get no response, creating a silent failure that makes the assistant feel broken in voice- and image-heavy communities.

## What Changes

- Detect `m.room.message` events with `msgtype: m.audio` in the Matrix long-poll sync loop
- Download the audio file from the Matrix content repository (`/_matrix/media/v3/download/...`)
- Transcribe the audio using the existing `assistant-transcription` crate
- Forward the transcribed text into the assistant runtime as a normal user message
- Detect `m.room.message` events with `msgtype: m.image` in the sync loop
- Download image bytes from the Matrix content repository
- Emit the image as `ChannelContent::FileData` for the runtime
- Extend `channel_runner.rs` to convert image `FileData` into a `ContentBlock::Image` multimodal attachment passed to the orchestrator
- Reply in the Matrix room with the assistant's response (text only; no audio synthesis or image generation)

## Capabilities

### New Capabilities

- `matrix-voice-messages`: Receive `m.audio` and `m.image` Matrix messages, process voice via transcription and images via multimodal LLM attachments, and reply as text

### Modified Capabilities

<!-- none — no existing spec-level requirements are changing -->

## Impact

- **`crates/interface-matrix`**: `adapter.rs` and `client.rs` need media download support; event filtering handles `m.audio` and `m.image` msgtypes
- **`crates/runtime`**: `channel_runner.rs` extended to convert image-typed `ChannelContent::FileData` into `ContentBlock::Image` attachments for the orchestrator
- **`crates/transcription`**: Used as-is via its existing public API; no changes expected
- **`Cargo.toml`** (interface-matrix): Add `assistant-transcription` as a dependency
- **Configuration**: A transcription provider (Whisper/Ollama) must be configured for voice; images require a vision-capable LLM (no additional config key — uses existing LLM provider)
- **No API or Flutter changes** required
