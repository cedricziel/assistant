## Context

The assistant platform supports voice/audio in some places but not others. Matrix and Slack adapters can receive audio messages and transcribe them. The `voice-response` tool can synthesize TTS audio. But two critical gaps exist:

1. **Outbound audio never reaches messaging platforms.** The `voice-response` tool stores audio in an ephemeral `AudioStore` and the web-ui serves it over HTTP, but the channel runner has no mechanism to retrieve that audio and send it as a `FileData` attachment through adapters.

2. **Signal, Mattermost, and Nextcloud ignore audio entirely.** No transcription provider wiring, no inbound audio parsing, no outbound audio sending.

Matrix and Slack already have the inbound pattern established: adapter holds `Option<Arc<dyn TranscriptionProvider>>`, set via `with_transcription()`. Audio messages are downloaded, transcribed, and emitted as `[Voice message]: {transcript}` text. This change replicates that pattern to the remaining adapters and closes the outbound gap.

## Goals / Non-Goals

**Goals:**

- TTS audio from `voice-response` tool is delivered as audio file attachments on all messaging platforms.
- Inbound audio messages on Signal, Mattermost, and Nextcloud are transcribed and dispatched as text.
- Consistent `with_transcription()` pattern across all adapters.

**Non-Goals:**

- Streaming/real-time voice (WebRTC, etc.).
- Passing raw audio to multimodal LLMs.
- Changes to the web-UI or Flutter app.
- New TTS/STT provider implementations.

## Decisions

### 1. Outbound audio via `AudioStore` retrieval in channel runner

**Decision:** When the orchestrator emits an `OrchestratorEvent::AudioReady { audio_id }`, the channel runner retrieves the audio blob from `AudioStore` and sends it through the adapter as `ChannelContent::FileData`.

**Why:** The `AudioStore` already holds the synthesized audio with MIME type and data. The channel runner already sends outbound `FileData` attachments from `turn_result.attachments`. We need to bridge the `AudioReady` event to an attachment send.

**Alternative considered:** Having each adapter subscribe to `AudioReady` events independently. Rejected because it duplicates logic and the channel runner already owns the dispatch loop.

**Implementation:** The channel runner needs access to `Arc<AudioStore>`. After a turn completes, if the turn's event stream contained `AudioReady` events, retrieve each audio blob and send as `FileData`. This requires the orchestrator to collect `AudioReady` events and expose them on `TurnResult`, or the channel runner to tap the event stream.

**Preferred approach:** Add an `audio_attachments: Vec<Attachment>` field to `TurnResult`. In the orchestrator dispatch, when `AudioReady` is emitted, also retrieve the blob from `AudioStore` and append it to the turn result's attachments. This keeps the channel runner unchanged — it already sends `turn_result.attachments`.

### 2. Inbound transcription follows existing adapter-level pattern

**Decision:** Each adapter that handles audio does its own transcription inline, using the established `with_transcription()` builder pattern. No change to the channel runner's `content_to_dispatch`.

**Why:** Matrix and Slack already do this. Audio handling is platform-specific (different message formats, download mechanisms, size limits). Centralizing in channel runner would require a generic "audio download" abstraction that doesn't exist.

**Alternative considered:** Moving transcription to channel runner's `content_to_dispatch` so all adapters get it for free. Rejected because each platform has different audio message formats and download mechanisms.

### 3. Signal audio via signal-cli-rest-api attachments

**Decision:** Signal inbound audio arrives as base64-encoded attachments in the WebSocket message JSON. The adapter checks MIME type via `is_audio_mime()`, decodes the data, and transcribes. Outbound audio uses `POST /v1/send` with a base64 attachment field.

**Why:** signal-cli-rest-api exposes attachments inline in the WebSocket JSON payload. No separate download step needed.

### 4. Mattermost audio via file upload/download API

**Decision:** Mattermost inbound audio arrives as file attachments on posts. The adapter downloads file metadata, checks MIME type, downloads the file content, and transcribes. Outbound audio uses the Mattermost file upload API followed by a post with file IDs.

**Why:** Mattermost uses a file-centric API where attachments are uploaded separately and referenced by ID in posts.

### 5. Nextcloud Talk audio via share/attachment API

**Decision:** Nextcloud Talk supports file sharing in chat rooms. Inbound audio files are detected via the message type and downloaded. Outbound audio uses the file upload + share-to-chat mechanism.

**Why:** Nextcloud Talk's webhook-based interface already receives rich message types. Audio files come through as shared files.

## Risks / Trade-offs

- **AudioStore TTL expiry** → If the TTS audio expires from the store before the channel runner retrieves it, delivery fails silently. Mitigation: retrieve immediately when the event fires; the default TTL (5 min) is generous for a synchronous turn.

- **Large audio files on constrained platforms** → Signal and Mattermost may have upload size limits. Mitigation: TTS responses are typically short (< 1 MB); document limits.

- **Transcription latency on slow providers** → Inbound audio transcription blocks the message dispatch. Mitigation: this is the existing behavior on Matrix/Slack and hasn't been problematic.

- **signal-cli-rest-api attachment format** → The base64 attachment format may vary across versions. Mitigation: test against the documented API; handle missing fields gracefully.

## Open Questions

- Should outbound audio be opt-in per adapter (config flag) or always sent when available? Current design: always send if TTS produced audio.
- Should the `ChannelContent::Voice` variant (currently unused) be repurposed or removed? Recommend removing it in a follow-up cleanup.
