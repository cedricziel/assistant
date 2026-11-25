# OpenRouter as a Fully Capable Provider

## Why

OpenRouter can serve every model call the assistant makes: chat, reasoning, web search, PDFs, structured output, images, speech-to-text, text-to-speech and embeddings. Today it is only a generic chat endpoint. The typed client drops reasoning, cost, routing and citations. PDFs are discarded. Embeddings fail. Voice and images need other vendors. One OpenRouter key should cover everything.

## What Changes

- **Chat:** a dedicated client keeps OpenRouter fields intact, handles OpenRouter errors including mid-stream ones, honors `Retry-After`, and sends attribution headers.
- **Model catalog:** supplies the context window, vision, PDF, tool, reasoning and output support.
- **Routing:** provider preferences, fallback models, and per-conversation `session_id` and hashed `user`.
- **Reasoning:** effort and budget settings. Reasoning streams as thinking and is kept across tool calls and restarts.
- **Prompt caching:** cache breakpoints, and cached-token accounting.
- **Web search:** a hosted `web` plugin, with Sources kept in the answer.
- **PDFs:** sent as `file` parts, with a choice of parser engine.
- **Chat options (all providers):** `tool_choice`, `parallel_tool_calls`, response format and output modalities, with each provider declaring what it supports.
- **Structured output (all providers):** native JSON schema, then a forced tool call, then a validated prompt. The skill judges switch to it.
- **Images:** an `[images]` provider and an `image-generate` tool for creating and editing images. Image-capable chat models can answer with images.
- **Voice-native chat:** audio-capable chat models hear voice messages directly, with the transcript alongside, and reply with speech. TTS is the fallback.
- **Usage:** cached, cache-write and reasoning tokens plus cost, in metadata and OTel.
- **Transcription:** an `openrouter` STT provider. Any ffmpeg-decodable input is accepted, long recordings are chunked, and the detected language is reported.
- **Speech:** an `openrouter` TTS provider with mp3, wav, ogg/Opus or PCM output for every model, transcoding when needed, and checks against the catalog.
- **Embeddings:** an `openrouter` provider with batching, plus per-provider default embedding models.
- **One key:** every OpenRouter feature reuses the `[llm]` key. `doctor` checks the key and shows the remaining credit.

## Non-goals

- A chat-model permission decider. Jev stays the decision model.
- Flutter settings UI for these options.

## Capabilities

### New Capabilities

`openrouter-chat`, `openrouter-model-metadata`, `openrouter-reasoning`, `openrouter-prompt-caching`, `openrouter-web-search`, `openrouter-documents`, `llm-chat-options`, `llm-structured-output`, `image-generation`, `openrouter-chat-media-output`, `openrouter-chat-audio-input`, `llm-usage-accounting`, `transcription-format-conversion`, `openrouter-transcription`, `openrouter-speech`, `openrouter-embeddings`, `openrouter-key-reuse`.

### Modified Capabilities

- `voice-receive`: on-demand TTS returns the configured format instead of always mp3.

`image-outbound`, `channel-audio-outbound` and `voice-send` already cover delivery. Their requirements do not change.

## Impact

- **Crates:** `core`, `llm-provider`, `transcription`, `runtime`, `storage` (one migration), `tool-executor`, `interfaces`, `web-ui`, `interface-cli`.
- **ffmpeg:** needed for transcoding and chunking.
- **API and UI:** no HTTP route or UI changes. Images and audio use the existing attachment rendering, so screenshot baselines stay the same.

**User-facing documentation: needed.** Rewrite `docs/openrouter.md`. Update `docs/transcription.md`, `docs/tool-permissions.md` and `docs/operations/configuration.md`, and add `docs/images.md`.
