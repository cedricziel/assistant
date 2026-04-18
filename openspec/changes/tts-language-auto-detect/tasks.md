## Tasks

### Phase 1: Fix TTS (immediate)

- [ ] Change `DEFAULT_MODEL` in `crates/transcription/src/deepgram_tts.rs` from `"aura-2-en-us"` to `"aura-2-thalia-en"`
- [ ] Update existing unit test `synthesize_returns_audio_bytes_on_success` to reflect new default
- [ ] Deploy and verify TTS works on schorschvm

### Phase 2: Language auto-detection

- [ ] Implement `detect_language(text: &str) -> &str` function in `deepgram_tts.rs` using stop-word frequency heuristic
- [ ] Add stop-word lists for English, German, Spanish, French, Japanese (Unicode script check)
- [ ] Add default voice mapping: `HashMap<&str, &str>` with `en → aura-2-thalia-en`, `de → aura-2-julius-de`, etc.
- [ ] Add `voices: HashMap<String, String>` field to `DeepgramTtsProvider` for config overrides
- [ ] Add `with_voices(map)` builder method on `DeepgramTtsProvider`
- [ ] In `synthesize()`: when `TtsRequest.voice` is `None`, call `detect_language()` and look up voice from map
- [ ] Write unit tests for language detection: English text, German text, mixed text, short text, empty text
- [ ] Write unit test: voice map override takes precedence over default
- [ ] Write unit test: explicit `TtsRequest.voice` overrides auto-detection

### Phase 3: Configuration

- [ ] Parse optional `[tts.voices]` table from `config.toml` in server startup code
- [ ] Pass voice map to `DeepgramTtsProvider::new().with_voices(map)` during initialization
- [ ] Update `config.toml` documentation / example with `[tts.voices]` section
- [ ] Verify backwards compatibility: existing configs without `[tts.voices]` use defaults

### Phase 4: Client error handling

- [ ] In `AudioPlayerWidget`: replace silent `catch` with error state (`_AudioState.error`)
- [ ] Render error state: error icon + "Failed to load audio" tooltip, tappable to retry
- [ ] Test: simulate TTS failure (e.g. disconnect server), verify error state appears
- [ ] Test: tap retry after error, verify it re-fetches audio
