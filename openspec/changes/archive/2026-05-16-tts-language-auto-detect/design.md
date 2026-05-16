## Context

TTS "read aloud" fails on every message. The server logs show:

```
WARN TTS synthesis failed: Deepgram TTS API returned 403 Forbidden:
     {"err_code":"INSUFFICIENT_PERMISSIONS",
      "err_msg":"Project does not have access to the requested model."}
```

The root cause: `deepgram_tts.rs` uses `DEFAULT_MODEL = "aura-2-en-us"` (line 11), which is not a valid Deepgram model name. The correct format is `aura-2-{voice}-{lang}` (e.g. `aura-2-thalia-en`, `aura-2-julius-de`).

Verified via API:

- `aura-2-en-us` → 403 (invalid model name)
- `aura-2-thalia-en` → 200 OK, valid MP3
- `aura-2-julius-de` → 200 OK, valid MP3

The assistant converses in multiple languages (at minimum English and German). A single hard-coded voice would sound wrong for non-matching languages. Auto-detecting the message language and selecting an appropriate voice provides a natural experience.

## Goals / Non-Goals

**Goals:**

- Fix the default model name so TTS works at all
- Auto-detect the language of each message and select a matching Deepgram voice
- Support at minimum English and German; fall back to English for unsupported languages
- Make the voice mapping configurable via `config.toml` for user overrides
- Surface TTS errors to the user in the Flutter client instead of silent failure

**Non-Goals:**

- Supporting every language Deepgram offers (add as needed)
- User-selectable voices in the UI (use config file for now)
- Streaming TTS (Deepgram supports it, but out of scope)
- Changing the TTS provider abstraction to be language-aware across all providers (Deepgram-specific for now)

## Decisions

### D1: Language detection via Unicode script analysis + stop-word heuristic

**Choice:** Detect language from the message text using a lightweight heuristic, not an external API or heavy NLP library:

1. Check Unicode script ranges for non-Latin scripts (Japanese → `ja`, etc.)
2. For Latin-script text, count occurrences of high-frequency stop words per language:
   - English: "the", "is", "and", "to", "of", "in", "that", "it"
   - German: "der", "die", "das", "und", "ist", "ein", "nicht", "den", "es"
   - Spanish: "el", "la", "los", "las", "de", "en", "que", "es"
   - French: "le", "la", "les", "de", "des", "un", "une", "et", "est"
3. The language with the highest stop-word density wins. Tie or no matches → default to English.

**Why:** No external dependency, sub-millisecond performance, good enough for full sentences. The assistant's responses are typically 50+ words, giving the heuristic plenty of signal. A crate like `whatlang` could also work but adds a dependency for something achievable in ~50 lines.

**Alternative considered:** Use the `whatlang` crate for proper trigram-based detection. Viable fallback if the heuristic proves unreliable — can be swapped in later without changing the interface.

### D2: Default voice map with config overrides

**Choice:** Hard-code a default voice per supported language. Allow overrides via `[tts.voices]` in `config.toml`:

```rust
// Default mapping
const DEFAULT_VOICES: &[(&str, &str)] = &[
    ("en", "aura-2-thalia-en"),
    ("de", "aura-2-julius-de"),
    ("es", "aura-2-lucia-es"),
    ("fr", "aura-2-chloe-fr"),
    ("ja", "aura-2-sakura-ja"),
];
```

```toml
# config.toml — optional overrides
[tts.voices]
en = "aura-2-zeus-en"
de = "aura-2-aurelia-de"
```

**Why:** Sensible defaults mean zero configuration for most users. Power users can pick their preferred voice per language. The config is read once at startup and stored on the `DeepgramTtsProvider`.

### D3: Language detection happens in `DeepgramTtsProvider.synthesize()`

**Choice:** The provider itself detects language from `TtsRequest.text` and selects the voice. The caller (web-ui API handler) doesn't need to know about language.

**Why:** Keeps language detection as an implementation detail of the Deepgram provider. Other TTS providers (e.g. a future local Sherpa-ONNX provider) may have different language handling. The `TtsProvider` trait stays unchanged.

**Flow:**

```
synthesize(TtsRequest { text, voice: None })
  → detect_language(text) → "de"
  → voices["de"] → "aura-2-julius-de"
  → POST /v1/speak?model=aura-2-julius-de
```

If `voice` is explicitly set in `TtsRequest`, it overrides auto-detection (existing behaviour preserved).

### D4: Surface errors in Flutter client

**Choice:** In `AudioPlayerWidget`, replace the silent `catch` with a `setState` that shows an error icon and tooltip. Tapping the error icon retries.

**Why:** Currently `fetchAudio` failures result in the loading spinner disappearing with no feedback. The user has no idea why "read aloud" doesn't work. A visible error state with retry is the minimum viable UX.

### D5: Fix DEFAULT_MODEL as the immediate first step

**Choice:** Change `DEFAULT_MODEL` from `"aura-2-en-us"` to `"aura-2-thalia-en"` as the first task, deployable independently of language detection.

**Why:** This single-line fix makes TTS work for English immediately. Language detection is an enhancement on top.

## Risks / Trade-offs

- **Heuristic accuracy:** Stop-word detection may misclassify short messages (< 10 words) or mixed-language messages. Mitigation: default to English on low confidence. For the assistant's responses (typically 50+ words), accuracy should be high.
- **New languages:** Adding a language requires adding stop words + a Deepgram voice to the default map. Low effort but manual.
- **Config parsing:** `[tts.voices]` is a new config section. Existing configs without it should work with defaults. Backwards compatible.

## Migration Plan

1. Deploy the `DEFAULT_MODEL` fix immediately — TTS starts working for English
2. Deploy language detection + voice map — TTS becomes language-aware
3. Users can optionally add `[tts.voices]` to their `config.toml` for custom voices
