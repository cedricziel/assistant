# Design

## Context

- `OpenRouterProvider` wraps `ChatCompletionsProvider`, which is built on the typed `async-openai` client. That client cannot send `reasoning`, `plugins`, `provider`, `models`, `session_id`, `file` parts or `cache_control`. It also drops `reasoning_details`, `annotations` and `usage.cost` from responses. `messages.rs` maps `ContentBlock::Document` to nothing.
- Provider construction is synchronous: `OrgContextCache` calls `build_llm` while holding a std mutex. `Orchestrator::new` reads `capabilities().context_window_tokens` once.
- Assistant tool calls are persisted as `Vec<ToolCallItem>` JSON in `messages.tool_calls_json`, and `history.rs` rebuilds `ChatHistoryMessage::AssistantToolCalls` from that JSON. Nothing provider-specific survives the round trip.
- The `LlmProvider` methods do not receive the conversation or the user.
- `build_provider` and `build_tts_provider` are synchronous and see only their own section. Both call sites are in async functions.
- `converter.rs` already shells out to ffmpeg for Deepgram.

## Goals / Non-Goals

**Goals:** OpenRouter behaves like a first-class provider. Its JSON round-trips without loss. Reasoning survives tool loops and restarts. Media formats are normalized centrally, so every provider benefits.

**Non-Goals:**

- Moving Moonshot off `ChatCompletionsProvider`.
- Reusing parsed-PDF `file` annotations. That needs provider state on plain assistant text messages, and it only saves parser cost. It is a possible follow-up.
- Live voice and model pickers in the UI.

## Decisions

### D1. A dedicated OpenRouter wire client (`llm-provider/src/openrouter/`)

The client uses `reqwest` with `serde` structs that model OpenRouter's schema. Fields we do not interpret are kept with `#[serde(flatten)] extra: Map` where they must round-trip. The client splits into modules: `wire.rs` (request and response types), `stream.rs` (an SSE parser that skips comments, handles mid-stream `error` and accumulates `reasoning_details`, tool-call deltas and usage), `errors.rs` (envelope to `anyhow` error with status, `error_type` and `Retry-After`), `catalog.rs`, `request.rs` (builds messages, plugins, routing and cache breakpoints) and `mod.rs` (the `LlmProvider` impl). `ChatCompletionsProvider` stays for Moonshot.

_Alternative:_ extend `async-openai` requests with extra fields through a raw-body escape hatch. Rejected because responses would still be lossy, and we would maintain two request shapes.

### D2. Retry policy honors `Retry-After`

The client extends `retry.rs` with an error type that carries `retry_after: Option<Duration>` and whether the error is transient. `with_retry` sleeps `max(backoff, retry_after)`. OpenRouter chat, transcription, speech and embeddings all use it. Other providers still get the same jittered backoff.

### D3. Per-call identity through a task-local `LlmCallContext`

`assistant_core::llm::LlmCallContext { conversation_id, user_hash }` lives in a `tokio::task_local!`. The orchestrator sets it around each turn's LLM calls with `LlmCallContext::scope(ctx, fut)`. The OpenRouter client reads it with `try_with`. When it is absent (background jobs), the client omits `session_id` and `user`. `user_hash` is a keyed hash (HMAC-SHA256 with the install's `jwt_secret`, truncated to 32 hex characters) of `org_slug:user_id`, so it stays stable but cannot be reversed off the box.

_Alternative:_ add a context parameter to `chat` and `chat_streaming`. Rejected for now because it touches every provider and around 10 test fakes for two optional fields. The task-local only requires that LLM calls are not `tokio::spawn`ed away from the turn future, which holds today. A test pins that invariant.

### D4. Opaque provider state on assistant tool calls

`ToolCallResponse` and `ChatHistoryMessage::AssistantToolCalls` gain `provider_state: Option<ProviderState>`, where `ProviderState { provider: String, model: String, data: serde_json::Value }`. The variant becomes the struct variant `AssistantToolCalls { items, provider_state }`, and all 49 match sites are updated. Storage gains a nullable `messages.provider_state_json` column (a new `space.db` migration). `history.rs` restores the column. The OpenRouter client writes `{"reasoning_details": [...]}` and sends it back only when `provider == "openrouter"`. Other providers ignore foreign state, which satisfies the provider-switch scenario. Anthropic thinking signatures can use the same slot later.

_Alternative:_ encode the state inside the first `ToolCallItem`. Rejected because it is per-turn data, not per-call data, and it would leak into tool-call telemetry and the UI.

### D5. Catalog loads asynchronously, and capabilities are read per turn

`OpenRouterCatalog` holds a `reqwest` client and an `arc_swap::ArcSwap<CatalogSnapshot>` (adding the `arc-swap` workspace dependency). `capabilities()` is synchronous, so it needs a lock-free read and must not block an async lock. Construction stays synchronous and spawns the fetch on the current runtime. The first `chat` call awaits a `tokio::sync::OnceCell`-backed readiness signal with a 5-second timeout. A failed fetch schedules a retry no sooner than 10 minutes later. `Orchestrator` moves the context-window adjustment from `new` into per-turn compaction setup, so late or changed values apply. Speech-model lookups use the same catalog client, which also calls `/audio/speech/models/{model}`.

_Alternative:_ make `build_llm` async. Rejected because the org cache holds a std mutex across construction, and making that async is a larger, unrelated refactor.

### D6. Reasoning config is validated twice

Statically, `serde` rejects `effort` combined with `max_tokens`. Against the catalog, an unsupported effort makes every chat call fail with the supported list, and startup logs it. Streaming maps `reasoning.text` and `reasoning.summary` to `StreamChunk::Thinking` and skips `reasoning.encrypted` for display. All entries are kept for D4.

### D7. Prompt-cache placement by model author

The model author is the slug prefix before `/`. `anthropic`, `google` and `qwen` get explicit breakpoints. Anthropic also gets a breakpoint on the last user part, which stays within Anthropic's limit of 4. Placement is a pure function of (author, messages), tested table-style.

### D8. Web search is a hosted tool, and citations are post-processed

When web search is enabled, `capabilities().hosted_tools` includes `HostedTool::WebSearch`, and the runtime already suppresses the local tool for it. Citation rendering is a pure function from (answer, annotations) to answer. In streaming, the Sources block is emitted as a final text chunk, so the persisted answer matches what was streamed.

### D9. Usage fields live in core, and OTel attributes live in runtime

`LlmResponseMeta` gains four `Option` fields. `otel_spans.rs` and `metrics.rs` record them. Anthropic (`cache_read_input_tokens`, `cache_creation_input_tokens`) and OpenAI (`cached_tokens`, `reasoning_tokens`) are filled in the same PR, because their APIs already report these values.

### D10. A central audio pipeline in `transcription`

`converter.rs` becomes a general `AudioPipeline` with `probe_duration` (ffprobe), `to_wav_16k_mono`, `split(max_secs)` and `transcode(src_fmt, pcm_rate, dst_fmt)`. `ConvertingTranscriber<P>` decorates any `TranscriptionProvider` that declares `accepted_formats()`. It converts only when needed, and Deepgram's ad-hoc conversion is removed in favor of it. Chunking is specific to OpenRouter, because the 60-second upstream timeout is an OpenRouter constraint. It lives in the OpenRouter STT provider and uses the pipeline. TTS transcoding uses `transcode`. Ogg output is `libopus` at 48 kHz, mono, 32 kbps, which matches messenger voice notes.

### D11. Async builders that take `&LlmConfig`

`build_provider(cfg, llm).await` and `build_tts_provider(cfg, llm).await` are async, so the TTS catalog validation can run, and they take `[llm]` for key reuse. `resolve_openrouter_key(section, llm)` in core implements the key order. The decider gets `JevDecider::from_config(cfg, llm)`. The old signatures are removed.

### D12. `embedding_model: Option<String>`

Each provider applies its own default (see the `openrouter-embeddings` spec).

### D13. `ChatOptions` through a new trait method with a default

`LlmProvider` gains `chat_with(system, history, tools, opts: &ChatOptions, sink)` and `chat_options_support() -> ChatOptionsSupport`. The default `chat_with` implementation has two cases:
- Options are at their defaults: delegate to `chat_streaming`.
- `tool_choice` is `required` or a specific tool on a provider without native support: apply the prompt-and-retry fallback.

Every in-tree provider overrides `chat_with` where its API has native fields. `chat` and `chat_streaming` stay as thin wrappers that pass default options, so the roughly 10 test fakes keep compiling unchanged.

*Alternative:* change the signatures of `chat` and `chat_streaming`. Rejected because it churns every fake for no behavior gain.

### D14. Structured output lives in core and uses a fallback chain

`assistant_core::llm::structured::chat_structured(llm, system, history, schema) -> Value` picks a strategy from `chat_options_support()`:
1. A native schema (`ResponseFormat::JsonSchema`).
2. A forced synthetic tool named `respond`, whose parameters are the schema.
3. Prompt embedding.

It validates with the existing workspace dependency `jsonschema` and retries once with the errors. OpenRouter adds the `response-healing` plugin for native, non-streaming calls. The skill judges call `chat_structured` with schemas defined next to their prompts.

### D15. Generated media on `LlmResponseMeta`

`LlmResponseMeta` gains `media: Vec<GeneratedMedia>`, where `GeneratedMedia { kind: Image | Audio, mime: String, data: bytes::Bytes }`. `Bytes` keeps cloning cheap. `bytes` is added as a workspace dependency; it is already in the dependency tree through `reqwest`. After a final answer, the orchestrator stores the media through `AttachmentStore` and links it to the assistant message. That is the same path tool attachments take, so the web, Slack, Matrix, Signal and Nextcloud adapters deliver media without changes.

### D16. Image generation is a separate provider and tool

`ImageGenerationProvider` (in a new `crates/llm-provider/src/images/` module) has `generate(ImageRequest) -> ImageResult` and `name()`. Its config is `[images]`, built in `space_runtime` like TTS. The `image-generate` handler in `tool-executor` resolves `reference_attachment_ids` through the attachment store, checks that each is an image, and returns `ToolOutput::with_attachment` for every image produced. Model validation uses `/images/models`, through the same catalog client as D5.

### D17. Voice turns and native audio replies

A voice turn is a turn whose `attachments` contain the new `ContentBlock::Audio { media_type, data }`. The web-ui voice send path and the Signal, Matrix, Mattermost and Nextcloud voice handlers attach the original recording next to the transcript text. This needs no new turn parameter. The orchestrator turns a voice turn plus provider audio support into `ChatOptions.output.audio = Some(voice)`. The OpenRouter client forces streaming for those calls and accumulates the PCM. It transcodes through D10's `AudioPipeline` to `[tts] format`, and attaches the result as `GeneratedMedia::Audio`. A 400 that names audio or modalities flips an `AtomicBool` that disables audio for that provider instance, and the call is repeated once without audio. When a voice turn's final answer has no audio and a TTS provider exists, the orchestrator synthesizes the clip with that TTS provider.

### D18. Audio input uses `ContentBlock::Audio`

The orchestrator decides per call whether to keep audio blocks. It keeps them only for the current turn's user message, and only when the provider's `chat_options_support()` reports audio input. Otherwise it strips them. This is the same place images are filtered for non-vision models. `history.rs` drops `Audio` blocks when it rebuilds past messages, so audio is never replayed. The recording stays in the `AttachmentStore`. The OpenRouter request builder converts any format outside `audio_input.formats` to WAV through the `AudioPipeline`, and adds the transcript part. A 400 that names `input_audio` flips an `AtomicBool` and repeats the call once without the audio. Every other provider's message builder treats `Audio` like an unsupported block and drops it, as a second guard. A test asserts the orchestrator stripped the block first.

*Alternative:* skip transcription when the model hears the audio. Rejected because the UI transcript bubble, memory, search and compaction all need text. Transcription stays the text of record.

## Risks / Trade-offs

- [The task-local context is lost if someone later spawns LLM calls] → A test asserts that `session_id` is present for a turn driven through the orchestrator. Its doc comment states the constraint.
- [The 49 `AssistantToolCalls` match sites make the D4 diff wide] → D4 lands as its own mechanical refactor PR before any behavior that uses it.
- [The catalog API changes shape] → Parsing is tolerant: unknown fields are ignored, and missing fields fall back to conservative values. Catalog failures never block chat.
- [Transcoding adds latency and needs ffmpeg] → Transcoding happens only when the model cannot produce the requested format. A missing ffmpeg is reported at build time.
- [Large base64 bodies] → Chunking bounds each audio request, and PDFs are bounded by the existing attachment size limits.
- [The `user` hash could still be correlated across orgs] → Hashing `org_slug:user_id` gives each org its own hashes.
- [OpenRouter does not document the PCM16 sample rate for chat audio] → The `pcm_sample_rate` default is 24000, which matches OpenAI audio models, and it can be configured. A recorded-fixture test pins the decode path.
- [OpenRouter does not document tool calls combined with audio output] → The provider repeats the call once without audio, falls back to TTS, and stops requesting audio after a rejection.
- [Audio input roughly doubles the cost of a voice turn (transcription plus audio tokens)] → Audio input is opt-in and capped by `max_secs`, and audio is never replayed in later turns.
- [Whether chat image output also arrives in stream deltas is undocumented] → Images are collected from both deltas and the final message, without duplicates.

## Migration Plan

One additive `space.db` migration (a nullable column). Config changes are additive, except that `[llm] embedding_model` becomes optional, and existing TOML still parses. Rollback means reverting the PRs. The extra column is harmless to older code.

## Delivery (stacked PRs)

1. Key-resolution helper and decision-model key reuse.
2. `Retry-After`-aware retry, plus extended usage in `LlmResponseMeta` and OTel (all providers).
3. `ProviderState` refactor and storage migration (no behavior change).
4. `LlmCallContext` task-local, plus per-turn context-window read.
5. `ChatOptions` trait method, native `tool_choice` and `parallel_tool_calls` for Anthropic, OpenAI, Moonshot and Ollama, the prompt fallback, and `[llm] parallel_tool_calls`.
6. Structured output in core, with skill judges migrated (Anthropic, OpenAI and the fallbacks).
7. OpenRouter wire client: chat, streaming, errors, attribution, routing, fallbacks, identity, and native chat options and structured output.
8. Catalog and capabilities, including chat-option support.
9. Reasoning.
10. Prompt caching.
11. Web search and citations.
12. PDFs.
13. Embeddings.
14. Audio pipeline and `ConvertingTranscriber`.
15. OpenRouter transcription.
16. OpenRouter TTS and `voice_response` format.
17. `GeneratedMedia` on responses, plus orchestrator persistence and delivery.
18. Image-generation provider and `image-generate` tool.
19. Chat image output.
20. Voice turns (`ContentBlock::Audio`), chat audio input, and native audio replies with TTS fallback.
21. `doctor` key health, plus the final docs pass.
