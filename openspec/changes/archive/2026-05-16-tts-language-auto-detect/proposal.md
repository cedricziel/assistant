## Why

TTS "read aloud" fails on every message. The Deepgram TTS provider uses `aura-2-en-us` as the default model, which is not a valid Deepgram model name. The correct format is `aura-2-{voice}-{lang}` (e.g. `aura-2-thalia-en`). Additionally, the assistant converses in multiple languages (English, German), so TTS should automatically select a voice matching the message language.

Confirmed via server logs:

```
WARN TTS synthesis failed: Deepgram TTS API returned 403 Forbidden:
     {"err_code":"INSUFFICIENT_PERMISSIONS",
      "err_msg":"Project does not have access to the requested model."}
```

Confirmed fix: `aura-2-thalia-en` and `aura-2-julius-de` both return 200 OK with valid MP3 audio.

## What Changes

- Fix the default model name from `aura-2-en-us` to `aura-2-thalia-en`
- Add language auto-detection to the TTS synthesis flow: detect the language of the message text and select an appropriate voice
- Support at least English and German voice selection; fall back to English for unsupported languages
- Make the voice mapping configurable in `config.toml` under `[tts]` (optional override per language)
- Surface TTS errors to the user in the Flutter client instead of silently swallowing them

## Capabilities

### New Capabilities

- `tts-language-detection`: Automatically detect message language and select a matching Deepgram voice model
- `tts-error-feedback`: Show a brief error message in the UI when TTS synthesis fails

### Modified Capabilities

- `tts-synthesis`: Fix default model, add language-aware voice selection

## Impact

### Backend (Rust)

- `crates/transcription/src/deepgram_tts.rs` — Fix `DEFAULT_MODEL`, add language detection + voice mapping logic
- `crates/transcription/src/provider.rs` — Optionally extend `TtsRequest` with a `language` hint
- `crates/core/src/types.rs` — Add language-to-voice mapping config if making it configurable
- `crates/web-ui/src/api/mod.rs` — Pass language hint or message content language to TTS provider

### Frontend (Flutter)

- `app/lib/features/chat/audio_player_widget.dart` — Surface errors to user instead of silent catch
- `app/lib/features/chat/chat_screen.dart` — Show snackbar or inline error when audio fetch fails

### Configuration

- `config.toml` — Optional `[tts.voices]` table for per-language voice overrides:
  ```toml
  [tts]
  provider = "deepgram"
  api_key = "..."
  # Optional: override default voices per language
  # [tts.voices]
  # en = "aura-2-thalia-en"
  # de = "aura-2-julius-de"
  ```
