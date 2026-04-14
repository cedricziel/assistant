## Context

The assistant already has a complete STT pipeline (`assistant-transcription`) used by Slack and Signal to transcribe voice attachments into text messages. The web UI does not participate in this pipeline at all — `ApiState` has no transcription field, and the conversation message endpoint accepts only `{ message: String }`.

There is no TTS infrastructure anywhere in the codebase. The OpenAI API (already used for Whisper STT) exposes a TTS endpoint (`POST /v1/audio/speech`) at the same base URL, making it the zero-friction first implementation. Deepgram (already implemented for STT) similarly has a TTS API (`POST /v1/speak`).

The Flutter app targets web (Chrome) and macOS. The `record` package supports both via the browser MediaRecorder API (WebM/Opus output) and macOS AVAudioEngine. The server already lists `audio/webm` in `AUDIO_MIME_PREFIXES`, so no new format support is needed.

## Goals / Non-Goals

**Goals:**

- Web UI users can record and send voice messages (STT path)
- Any assistant message can be played back as synthesized speech on demand (TTS path)
- The assistant can proactively respond in voice via a dedicated tool
- TTS shares credentials/config with the existing transcription provider when using OpenAI or Deepgram
- Feature degrades gracefully when TTS is not configured (play button absent or disabled)

**Non-Goals:**

- Streaming TTS (real-time audio as the assistant generates text) — too complex for v1
- Other interface support for TTS (Slack, Signal, Matrix) — follow-on
- Speaker diarisation or emotion in voice responses
- Persistent audio storage (audio is ephemeral / generated on-demand)
- Wake-word or continuous listening

## Decisions

### D1: Extend `assistant-transcription`, not a new crate

**Decision:** Add `TtsProvider` trait, `TtsRequest`, `TtsResult`, and provider implementations to `assistant-transcription`. Add `build_tts_provider()` helper.

**Rationale:** TTS and STT share the same provider credentials (OpenAI key, Deepgram key, base URL). Bundling them avoids a third crate in the audio domain, and the crate can evolve to be the "audio AI" crate. Adding a second trait does not break existing callers.

**Alternative considered:** New `assistant-tts` crate. Rejected because it doubles the config boilerplate and creates another entry in the workspace dependency graph for no architectural benefit.

### D2: Separate `[tts]` config section

**Decision:** Add `TtsConfig` to `AssistantConfig` under key `[tts]`, structurally identical to `TranscriptionConfig` (provider, model, api_key, base_url, voice).

**Rationale:** Users may want OpenAI for TTS but Ollama for STT (or different voices/models). Keeping configs independent preserves that flexibility. The `config.toml` comment will note that api_key defaults to the same env var as transcription when using the same provider.

**Alternative considered:** Reuse `TranscriptionConfig` for TTS. Rejected because model names differ between STT and TTS (e.g., `whisper-1` vs `tts-1`) and adding a `voice` field to a struct named `TranscriptionConfig` is confusing.

### D3: In-memory `AudioStore` for tool-synthesized audio

**Decision:** An `AudioStore` is an `Arc<RwLock<HashMap<Uuid, (Vec<u8>, Instant)>>>` stored in `ApiState`. Entries expire after 1 hour. No persistence across restarts.

**Rationale:** Tool-synthesized audio is ephemeral. The `voice_response` tool is called once per interaction; the client fetches and plays immediately. Persistent storage (SQLite BLOB, disk) adds schema migration complexity and storage growth with no benefit. If the user misses auto-play they can tap the play button to re-synthesize via the on-demand endpoint.

### D4: On-demand synthesis for the play button

**Decision:** `GET /api/messages/{msg_id}/audio` reads the message text from the DB, synthesizes on each request, returns raw mp3 bytes. No caching.

**Rationale:** Keeps the server stateless with respect to audio. Re-synthesis cost is minimal for typical assistant message lengths. Caching can be added later if profiling shows it matters.

**Alternative considered:** Pre-synthesize all assistant messages and store alongside the message. Rejected because most messages may never be played; it wastes API quota and storage.

### D5: SSE `audio_ready` event for proactive voice responses

**Decision:** When the orchestrator processes a `voice_response` tool result, it emits a new SSE event type:

```
event: audio_ready
data: {"audio_id":"<uuid>","auto_play":true}
```

The Flutter client listens for this event and auto-fetches `GET /api/audio/{audio_id}`.

**Rationale:** The existing SSE stream (`text/event-stream`) already connects orchestrator to client. Adding a new named event is the lowest-friction way to push the audio_id. The Flutter `http` package's SSE client already supports named events.

**Alternative considered:** Return the audio_id in the final SSE `done` event. Rejected because the tool may fire mid-conversation before the final turn ends.

### D6: `voice_response` tool needs TTS provider in executor context

**Decision:** `VoiceResponseHandler` stores `Option<Arc<dyn TtsProvider>>` and `Arc<AudioStore>`, both injected at registration time via `ToolExecutor::register_builtins()`. If TTS is not configured, the tool is not registered (silently absent from the tool list).

**Rationale:** Matches the pattern for other context-dependent tools (e.g., memory tools that require a storage layer). The tool is simply not offered to the LLM when unavailable, so the assistant cannot mistakenly try to call it.

### D7: Flutter audio via `record` + `audioplayers`

**Decision:** Add `record: ^5.0.0` for capture and `audioplayers: ^6.0.0` for playback.

**Rationale:** Both packages have web and macOS support, active maintenance, and are the de-facto standard in the Flutter ecosystem. `record` outputs WebM/Opus on web (already in server's allowed MIME types) and M4A on macOS (also allowed).

**Alternative considered:** Browser Web Speech API via `dart:html`. Rejected for macOS: no native equivalent without a platform channel. Also bypasses the server's transcription providers (quality matters).

## Risks / Trade-offs

- **Microphone permission UX**: First use on web triggers a browser permission prompt; on macOS requires `NSMicrophoneUsageDescription` in Info.plist. Mitigation: add the plist key; on web the browser handles it natively.
- **TTS API cost**: Every play-button tap re-synthesizes. Mitigation: clearly document in config; add response caching as a follow-on if needed.
- **Audio store memory growth**: Many `voice_response` calls accumulate in-memory. Mitigation: 1-hour TTL + a background sweep task on a 10-minute tick.
- **Ollama has no TTS**: Users running fully-local setups cannot use voice responses. Mitigation: degrade gracefully (no play button); document clearly. A local TTS provider (e.g., Kokoro) can be added in a follow-on.
- **SSE client parsing**: The Flutter SSE client must handle named events (`event: audio_ready`). Mitigation: verify the existing `client.dart` SSE parser handles named events; fix if needed as part of this change.

## Migration Plan

1. Deploy new server binary — new `[tts]` config key is optional; existing deployments continue to function without it.
2. Users opt in by adding `[tts]` to `~/.assistant/config.toml`.
3. No database migrations required.
4. Flutter app update ships simultaneously; new UI elements (mic button, play button) are inert if the server has no TTS configured (endpoint returns 503).

## Open Questions

- **Voice selection UX**: Should the UI expose a voice picker (alloy/nova/echo/…) or fix the voice in config? → Fix in config for v1; UI picker is a follow-on.
- **Max recording length**: Whisper API limit is 25 MB / ~30 min. Should the client enforce a shorter cap (e.g., 2 min) for UX reasons? → Enforce 2-minute client-side limit with a visible timer.
