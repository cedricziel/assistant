## Context

The Matrix interface (`crates/interface-matrix`) currently handles only `m.room.message` events with `msgtype: m.text`. Voice messages (`msgtype: m.audio`) and images (`msgtype: m.image`) are silently dropped at adapter.rs:161. Both arrive with a `url` field containing an `mxc://` content URI.

The `assistant-transcription` crate provides a `TranscriptionProvider` trait (Whisper, Ollama, Deepgram) and is used identically by `interface-slack`. The pattern for voice is established: add a `with_transcription()` builder, detect audio events, download via the homeserver, transcribe, emit as text.

For images, `assistant-llm` already defines `ContentBlock::Image { media_type, data: base64 }`, and `Orchestrator::run_turn_with_tools` already accepts `attachments: Vec<ContentBlock>`. However, `channel_runner.rs` currently only dispatches `ChannelContent::Text` events — it must be extended to convert image-typed `ChannelContent::FileData` into a multimodal attachment for the orchestrator.

MXC URIs (`mxc://<server>/<media_id>`) resolve to `/_matrix/media/v3/download/<server>/<media_id>`.

## Goals / Non-Goals

**Goals:**

- Detect `m.audio` voice messages in the long-poll sync loop; transcribe and inject as text
- Detect `m.image` images in the long-poll sync loop; download bytes and emit as `ChannelContent::FileData`
- Extend `channel_runner.rs` to convert image `FileData` into `ContentBlock::Image` multimodal attachments for the orchestrator
- Resolve `mxc://` URIs and download media bytes via `MatrixClient`
- Follow the exact `with_transcription()` builder pattern used by `interface-slack`
- Surface a clear `warn!` when a voice message arrives but no transcription provider is configured

**Non-Goals:**

- Audio synthesis / text-to-speech replies
- Encrypted media (`m.room.encrypted`) — requires full Matrix E2E which we deliberately avoid
- Modifying the `assistant-transcription` crate
- Supporting `m.video` or `m.file` attachments (separate concern)
- Guaranteeing image understanding — depends on the configured LLM being vision-capable

## Decisions

### Reuse the Slack pattern verbatim (voice)

**Decision**: Mirror `interface-slack`'s `with_transcription(provider, language)` builder on both `MatrixAdapter` and `MatrixRunner`.  
**Rationale**: The same `Arc<dyn TranscriptionProvider>` pattern is already in the workspace and battle-tested.  
**Alternative considered**: Config-file-driven provider construction inside the adapter. Rejected: runner owns config-to-provider wiring.

### Media download via `MatrixClient::download_media`

**Decision**: Add `download_media(mxc_url: &str, max_bytes: usize) -> Result<(Vec<u8>, String)>` to `MatrixClient`, returning `(bytes, mime_type)`.  
**Rationale**: Keeps the bearer token co-located with the HTTP client; the same method serves both audio and image downloads.  
**Alternative considered**: Separate `download_audio` / `download_image` methods. Rejected: identical logic; one method reduces duplication.

### Images emitted as `ChannelContent::FileData`, not `ChannelContent::Image { url }`

**Decision**: The adapter downloads image bytes and emits `ChannelContent::FileData { data, filename, mime_type }`.  
**Rationale**: `ChannelContent::Image { url }` contains only a URL, which is not publicly accessible (mxc:// URIs require authenticated download). The orchestrator needs raw bytes. `FileData` is the correct variant for in-memory binary content.  
**Alternative considered**: Emitting `ChannelContent::Image { url: mxc_url }` and converting in the runner. Rejected: the runner has no Matrix credentials to re-download.

### Image dispatch extended in `channel_runner.rs`

**Decision**: Extend the `dispatch` function in `crates/runtime/src/channel_runner.rs` to recognise image-MIME `FileData` as a special case: base64-encode the bytes, build `ContentBlock::Image { media_type, data }`, and call `run_turn_with_tools` with `user_message = "[Image attached]"` and the image as attachment.  
**Rationale**: The orchestrator already accepts `Vec<ContentBlock>` attachments and vision-capable LLMs process them correctly. Keeping this logic in `channel_runner` means all adapters (Slack, Signal, etc.) benefit from the same extension for free once they emit `FileData`.  
**Alternative considered**: Doing the base64 conversion inside the Matrix adapter only. Rejected: other adapters may send images via `FileData` in future; this is the right seam.

### v3 Content Repository path

**Decision**: Use `/_matrix/media/v3/download/<server>/<media_id>` (Matrix spec ≥ v1.4).  
**Rationale**: v3 is the current spec path; servers from 2022+ support it.  
**Alternative considered**: v1/r0 for broader compatibility. Rejected: too old.

### Max download size

**Decision**: 25 MB cap for audio (Whisper limit); 10 MB cap for images (LLM context/cost concern).  
**Rationale**: Different constraints per media type; images rarely exceed a few MB but we apply a tighter guard for cost control.

### Transcript and image text framing

**Decision**: `[Voice message]: <text>` for audio; `[Image attached]` as the user_message text + `ContentBlock::Image` attachment for images.  
**Rationale**: Consistent with Slack for audio; the LLM receives structured multimodal input for images rather than a base64 blob embedded in text.

## Risks / Trade-offs

- **Unencrypted media only** → Encrypted rooms will still not handle voice or images. Acceptable given thin-client architecture.
- **Vision LLM not configured** → If the LLM provider does not support vision, `ContentBlock::Image` is silently discarded by the provider (existing behavior in `assistant-llm`); the LLM sees only `[Image attached]` with no context. Acceptable: operator responsibility.
- **MXC URI parsing** → Malformed URIs produce a `warn!` and are skipped.
- **Provider not configured (voice)** → `warn!` and drop — no crash.
- **v3 path availability** → [Risk] Very old homeservers may not support it. → Mitigation: log warning with HTTP status, skip gracefully.
- **Image size** → 10 MB guard prevents OOM; larger images are skipped with a warning.
