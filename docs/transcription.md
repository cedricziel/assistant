# Voice Transcription

Automatically transcribes audio attachments (voice messages, voice notes) into text before the agent processes them. The transcript is injected as `[Voice transcription]: ...` so the agent sees it as a normal text message.

## Supported providers

| Provider   | Type   | Default model            | API key env var    |
| ---------- | ------ | ------------------------ | ------------------ |
| `whisper`  | Hosted | `whisper-1`              | `OPENAI_API_KEY`   |
| `ollama`   | Local  | `whisper-large-v3-turbo` | _(none)_           |
| `deepgram` | Hosted | `nova-3`                 | `DEEPGRAM_API_KEY` |

## Configuration

Add to `~/.assistant/config.toml`:

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

## Supported audio formats

Ogg/Opus, MP3, M4A, WAV, FLAC, WebM, AAC. File size limit is 25 MB (matches the Whisper API limit).

## How it works

1. An interface (Slack, Signal, ...) receives a message with audio file attachments.
2. The audio bytes are downloaded and passed to the configured `TranscriptionProvider`.
3. The provider returns the transcript text.
4. The transcript is prepended to the user's message as `[Voice transcription]: <text>`.
5. The agent processes the combined text normally.

If transcription fails, a `[Voice message: transcription failed]` placeholder is inserted instead — the message is never silently dropped.

## Interface support

| Interface  | Status                                           |
| ---------- | ------------------------------------------------ |
| Slack      | Supported — voice messages and audio file shares |
| Signal     | Planned                                          |
| Mattermost | Planned                                          |
| CLI        | N/A                                              |

## Architecture

The `assistant-transcription` crate provides:

- **`TranscriptionProvider` trait** — `name()` and `transcribe(request) -> Result<TranscriptionResult>`
- **`WhisperProvider`**, **`OllamaTranscriptionProvider`**, **`DeepgramProvider`** — concrete implementations
- **`build_provider(config)`** — factory that creates a provider from `TranscriptionConfig`
- **`is_audio_mime(mime)`** — helper to detect audio MIME types

Providers are injected into interfaces via `Arc<dyn TranscriptionProvider>`, following the same pattern as `LlmProvider`.
