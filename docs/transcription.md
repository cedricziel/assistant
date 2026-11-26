# Voice Transcription

Automatically transcribes audio attachments (voice messages, voice notes) into text before the agent processes them. The transcript is injected as `[Voice transcription]: ...` so the agent sees it as a normal text message.

## Supported providers

| Provider     | Type   | Default model            | API key env var      |
| ------------ | ------ | ------------------------ | -------------------- |
| `whisper`    | Hosted | `whisper-1`              | `OPENAI_API_KEY`     |
| `ollama`     | Local  | `whisper-large-v3-turbo` | _(none)_             |
| `deepgram`   | Hosted | `nova-3`                 | `DEEPGRAM_API_KEY`   |
| `openrouter` | Hosted | `openai/whisper-1`       | `OPENROUTER_API_KEY` |

## Configuration

Add to `~/.assistant/orgs/default/org.toml`:

```toml
[transcription]
provider = "deepgram"
# api_key = "..."        # or set DEEPGRAM_API_KEY env var
# model = "nova-3"       # uses provider default if omitted
# base_url = "..."       # override for self-hosted / on-prem
# language = "en"        # optional BCP-47 language hint
```

### Whisper (OpenAI)

```toml
[transcription]
provider = "whisper"
# api_key via OPENAI_API_KEY env var or config
```

Compatible with any server exposing the OpenAI `/v1/audio/transcriptions` endpoint (LocalAI, vLLM, etc.) — set `base_url` to point at it.

### Ollama (local)

```toml
[transcription]
provider = "ollama"
# base_url = "http://localhost:11434/v1"
# model = "whisper-large-v3-turbo"
```

Requires a whisper-compatible model pulled locally:

```sh
ollama pull whisper-large-v3-turbo
```

No API key needed. All audio stays on your machine.

### Deepgram

```toml
[transcription]
provider = "deepgram"
# api_key via DEEPGRAM_API_KEY env var or config
# model = "nova-3"
```

### OpenRouter

```toml
[transcription]
provider = "openrouter"
# model = "openai/whisper-1"
# base_url = "https://openrouter.ai/api/v1"
# language = "en"           # sent as the language hint
# prompt = "Names: Acme"    # sent as the prompt
# max_chunk_secs = 50       # default 50, capped at 55
```

The key is `[transcription] api_key`, else the `[llm]` key when `[llm] provider = "openrouter"`, else `OPENROUTER_API_KEY`.

Each request posts JSON to `{base_url}/audio/transcriptions` with `model`, the audio base64-encoded in `input_audio`, `language` and `prompt` when set, and `response_format: "verbose_json"`. If the model rejects `verbose_json` with a 400, the request is retried with `json`, and that provider instance uses `json` from then on. A `json` response carries no detected language, so the result reports the configured `language` instead. Detected language names are normalized to codes, for example `english` to `en`.

Recordings longer than `max_chunk_secs` are split with ffmpeg and the pieces transcribed in order, joined with spaces. The cap is 55 seconds because upstreams time out after 60. If one piece fails, the whole transcription fails. Audio whose duration ffprobe can't read is sent whole. Chunking needs `ffmpeg` and `ffprobe` on `PATH`.

Errors carry the HTTP status and the upstream message. Transient errors are retried, honoring `Retry-After`. A 413 reports that the audio exceeds the upstream size limit.

## Supported audio formats

These MIME types are transcribed (see `AUDIO_MIME_TYPES`): Ogg/Opus, MP3, M4A/MP4 audio, WAV, FLAC, WebM, AAC, AMR, 3GPP, CAF, AIFF, and MP4, WebM and QuickTime video. Matching ignores case and parameters such as `; codecs=opus`. File size limit is 25 MB (matches the Whisper API limit).

### Conversion

Each provider declares the formats it accepts:

| Provider            | Accepted formats                    |
| ------------------- | ----------------------------------- |
| `deepgram`          | mp3, wav, ogg, flac, webm           |
| `whisper`, `ollama` | flac, m4a, mp3, mp4, ogg, wav, webm |
| `openrouter`        | wav, mp3, flac, m4a, ogg, webm, aac |

Any other recognized format is converted to 16 kHz mono WAV with ffmpeg before it is sent; for video, only the audio stream is kept. Unrecognized MIME types are sent unchanged. Conversion needs `ffmpeg` on `PATH`; without it, the transcription fails with an error naming ffmpeg and the format.

## How it works

1. An interface (Slack, Signal, ...) receives a message with audio file attachments.
2. The audio bytes are downloaded and passed to the configured `TranscriptionProvider`.
3. The provider returns the transcript text.
4. The transcript is prepended to the user's message (Slack uses `[Voice transcription]: <text>`; Signal uses `[Voice message]: <text>`).
5. The agent processes the combined text normally.

If transcription fails, the Slack interface inserts a `[Voice message: transcription failed]` placeholder instead — the message is never silently dropped.

## Interface support

| Interface  | Status                                           |
| ---------- | ------------------------------------------------ |
| Slack      | Supported — voice messages and audio file shares |
| Signal     | Supported                                        |
| Mattermost | Supported                                        |
| Matrix     | Supported                                        |
| Nextcloud  | Supported                                        |
| Web        | Supported — `POST /api/conversations/{id}/voice` |
| CLI        | N/A                                              |

## Architecture

The `assistant-transcription` crate provides:

- **`TranscriptionProvider` trait** — `name()` and `transcribe(request) -> Result<TranscriptionResult>`
- **`WhisperProvider`**, **`OllamaTranscriptionProvider`**, **`DeepgramProvider`**, **`OpenRouterTranscriber`** — concrete implementations
- **`ConvertingTranscriber`** — wraps every provider `build_provider` returns and converts audio the provider doesn't accept
- **`build_provider(config)`** — factory that creates a provider from `TranscriptionConfig`
- **`is_audio_mime(mime)`** — helper to detect audio MIME types

Providers are injected into interfaces via `Arc<dyn TranscriptionProvider>`, following the same pattern as `LlmProvider`.
