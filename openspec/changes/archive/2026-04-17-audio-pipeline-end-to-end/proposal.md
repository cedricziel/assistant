## Why

The audio pipeline has significant gaps: synthesized audio (TTS) from the `voice-response` tool cannot be delivered back through messaging platforms, incoming audio on Signal/Mattermost/Nextcloud is silently dropped, and the channel runner discards non-image `FileData` — severing the outbound audio path for all adapters. This change wires up the missing pieces so audio flows end-to-end across all supported platforms.

## What Changes

- **Channel runner**: Extend `content_to_dispatch` to handle audio `FileData` by transcribing it (when a provider is available) and dispatching the transcript as text, mirroring the Matrix/Slack adapter logic but at the generic layer.
- **Channel runner outbound audio**: When the `voice-response` tool produces audio, retrieve the blob from `AudioStore` and send it as a `FileData` attachment through the adapter — so messaging platforms can deliver synthesized speech.
- **Signal adapter**: Add transcription provider support and inbound audio message parsing; add outbound audio file sending via the signal-cli REST API.
- **Mattermost adapter**: Add transcription provider support, inbound audio file handling on `file_share`-style events, and outbound audio file upload.
- **Nextcloud adapter**: Add transcription provider support for inbound audio and outbound audio file sending via the Nextcloud Talk API.

## Non-goals

- Real-time voice streaming or WebRTC integration.
- Preserving raw audio as multimodal LLM input (audio is always transcribed to text).
- Changes to the web-UI or Flutter app — those paths already work.
- Adding new TTS/STT provider implementations.

## Capabilities

### New Capabilities

- `channel-audio-outbound`: Generic channel-runner logic to deliver TTS-synthesized audio blobs back through any adapter as `FileData` attachments.
- `signal-voice-messages`: Inbound audio transcription and outbound audio sending for the Signal adapter.
- `mattermost-voice-messages`: Inbound audio transcription and outbound audio sending for the Mattermost adapter.
- `nextcloud-voice-messages`: Inbound audio transcription and outbound audio sending for the Nextcloud adapter.

### Modified Capabilities

- `channel-runner`: Add audio `FileData` dispatch (inbound transcription at the runner level) and outbound `AudioStore` retrieval.

## Impact

- **Crates touched**: `runtime` (channel runner), `interface-signal`, `interface-mattermost`, `interface-nextcloud`, `core` (possibly extend `ChannelAdapter` trait with optional transcription provider).
- **Dependencies**: `assistant-transcription` becomes a dependency of the three new adapter crates.
- **No API changes**: No new HTTP endpoints; no OpenAPI spec changes.
- **No breaking changes**: Existing behavior is preserved; audio support is additive and gated on provider configuration.
