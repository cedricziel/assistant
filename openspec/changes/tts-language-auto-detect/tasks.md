## Tasks

### Phase 1: Fix TTS (immediate)

- [x] Change `DEFAULT_MODEL` in `crates/transcription/src/deepgram_tts.rs` from `"aura-2-en-us"` to `"aura-2-thalia-en"`
- [x] Update existing unit test `synthesize_returns_audio_bytes_on_success` to reflect new default
- [ ] Deploy and verify TTS works on schorschvm

### Phase 2: Language auto-detection

- [x] Implement `detect_language(text: &str) -> &str` function in `deepgram_tts.rs` using stop-word frequency heuristic
- [x] Add stop-word lists for English, German, Spanish, French, Japanese (Unicode script check)
- [x] Add default voice mapping: `HashMap<&str, &str>` with `en → aura-2-thalia-en`, `de → aura-2-julius-de`, etc.
- [x] Add `voices: HashMap<String, String>` field to `DeepgramTtsProvider` for config overrides
- [x] Add `with_voices(map)` builder method on `DeepgramTtsProvider`
- [x] In `synthesize()`: when `TtsRequest.voice` is `None`, call `detect_language()` and look up voice from map
- [x] Write unit tests for language detection: English text, German text, mixed text, short text, empty text
- [x] Write unit test: voice map override takes precedence over default
- [x] Write unit test: explicit `TtsRequest.voice` overrides auto-detection

### Phase 3: Configuration

- [x] Parse optional `[tts.voices]` table from `config.toml` in server startup code
- [x] Pass voice map to `DeepgramTtsProvider::new().with_voices(map)` during initialization
- [x] Update `config.toml` documentation / example with `[tts.voices]` section
- [x] Verify backwards compatibility: existing configs without `[tts.voices]` use defaults

### Phase 4: Client error handling

- [x] In `AudioPlayerWidget`: replace silent `catch` with error state (`_AudioState.error`)
- [x] Render error state: error icon + "Failed to load audio" tooltip, tappable to retry
- [x] Test: simulate TTS failure (e.g. disconnect server), verify error state appears
- [x] Test: tap retry after error, verify it re-fetches audio
