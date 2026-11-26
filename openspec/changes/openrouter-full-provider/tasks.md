# Tasks

Each numbered section is one stacked PR. Every PR ends with `make lint && make format && make test` passing and `openspec validate openrouter-full-provider --strict` clean.

## 1. Key resolution and decision model

- [x] 1.1 Write failing tests in `crates/core/src/types/llm.rs` for `LlmConfig::openrouter_api_key()` and `resolve_openrouter_key`. Cover section key, reused `[llm]` key, a non-OpenRouter `[llm]` key never reused, env fallback, and no key. Confirm red.
- [x] 1.2 Implement both helpers. Verify the 1.1 tests pass.
- [x] 1.3 Write failing tests for `DecisionModelConfig.api_key` parsing and for `JevDecider::from_config(cfg, llm)` key order. Confirm red.
- [x] 1.4 Add `api_key`, change `from_config`, and pass `&config.llm` from `build_permission_policy`. Verify with `cargo test -p assistant-runtime decision_model` and the permission tests.
- [x] 1.5 Update `docs/tool-permissions.md` and `docs/operations/configuration.md`. Check every claim against the code.

## 2. Retry-After and usage accounting

- [x] 2.1 Write a failing test for `with_retry`: it waits at least `retry_after` when the error carries one, and does not retry non-transient errors. Confirm red.
- [x] 2.2 Add a retry-aware error type to `retry.rs` and honor `retry_after`. Verify 2.1.
- [x] 2.3 Write failing tests: Anthropic and OpenAI responses fill `cached_input_tokens`, `cache_write_input_tokens` and `reasoning_tokens`; Ollama leaves `cost_usd` unset. Confirm red.
- [x] 2.4 Add the four `Option` fields to `LlmResponseMeta` and fill them in the Anthropic and OpenAI parsers. Verify 2.3.
- [x] 2.5 Write failing tests in `otel_spans.rs` and `metrics.rs` for the cache-read, cache-creation and reasoning attributes and `assistant.llm.cost_usd`. Confirm red. Implement, then verify.

## 3. Provider state refactor (no behavior change)

- [x] 3.1 Write a failing storage test: a message saved with `provider_state_json` round-trips, and older rows read back as `None`. Confirm red.
- [x] 3.2 Add the nullable `messages.provider_state_json` migration and the store field. Verify 3.1 with `cargo test -p assistant-storage`.
- [x] 3.3 Write a failing `history.rs` test: a stored provider state is restored onto `AssistantToolCalls`. Confirm red.
- [x] 3.4 Change `AssistantToolCalls` to `{ items, provider_state }`, add `provider_state` to `ToolCallResponse`, update all match sites, persist the state in the orchestrator, and restore it in `history.rs`. Verify with `cargo test --workspace`.

## 4. Per-call context and per-turn context window

- [x] 4.1 Write failing tests for `LlmCallContext::scope` and `try_current`, and for `user_hash` (stable, org-scoped, not equal to the raw ID). Confirm red. Implement in core, then verify.
- [x] 4.2 Write a failing orchestrator test: a scripted provider observes the conversation ID inside a turn and none in a background job. Confirm red. Wrap the turn LLM calls in `scope`, then verify.
- [x] 4.3 Write a failing test: a provider whose `context_window_tokens` changes after construction affects compaction on the next turn. Confirm red. Move the adjustment into per-turn setup, then verify.

## 5. Chat options (all providers)

- [x] 5.1 Write failing tests for `ChatOptions` defaults: `chat_with` with default options produces the same request as today for every provider (wiremock or scripted). Confirm red. Add `ChatOptions`, `ChatOptionsSupport`, the default `chat_with`, and `chat`/`chat_streaming` as wrappers. Verify.
- [x] 5.2 Write failing tests for native `tool_choice` and `parallel_tool_calls` mapping on Anthropic (`any`/`tool`, `disable_parallel_tool_use`) and OpenAI. Confirm red. Implement, then verify. Moonshot keeps the prompt fallback (see D13).
- [x] 5.3 Write failing tests for the prompt fallback on Ollama: a forced tool not called leads to one retry and then an error naming the tool; `none` sends no tools. Confirm red. Implement, then verify.
- [x] 5.4 Write failing tests: `[llm] parallel_tool_calls` parses, and the orchestrator passes it on agent-loop calls. Confirm red. Implement, then verify. Document in `docs/operations/configuration.md`.

## 6. Structured output (all providers)

- [x] 6.1 Write failing tests for `chat_structured` strategy selection: native schema, forced `respond` tool, then prompt embedding; driven by `ChatOptionsSupport` on a scripted provider. Confirm red. Implement, then verify.
- [x] 6.2 Write failing tests: validation through `jsonschema`, fenced JSON stripped, one repair retry carrying the errors, and a final error containing the errors and the raw output. Confirm red. Implement, then verify.
- [x] 6.3 Write failing wiremock tests for native JSON schema on OpenAI, and for the forced tool on Anthropic. Confirm red. Implement, then verify.
- [x] 6.4 Write failing tests: the skill learner and skill improver send schemas and accept fenced or chatty model output that the fallback path repairs. Confirm red. Migrate both judges, then verify.

## 7. OpenRouter wire client

- [x] 7.1 Write failing wiremock tests for non-streaming chat: request shape (messages, tools, `max_completion_tokens`), served model in meta, tool-call parsing, text answer, and usage including `cost`. Confirm red.
- [x] 7.2 Implement `wire.rs`, `request.rs` and non-streaming `chat`. Verify 5.1.
- [x] 7.3 Write failing SSE tests covering comment lines, text deltas, tool-call deltas, the final usage chunk, mid-stream `error` with `finish_reason: "error"`, and `[DONE]`. Confirm red. Implement `stream.rs`, then verify.
- [x] 7.4 Write failing error tests for the 402, 403 moderation reasons, 429 with `Retry-After`, 503 retried, 400 not retried, and a 200 body with `error`. Confirm red. Implement `errors.rs` on top of task 2, then verify.
- [x] 7.5 Write failing tests for the `HTTP-Referer`, `X-OpenRouter-Title` and `X-OpenRouter-Categories` headers, and config rejection of more than two categories. Confirm red. Implement, then verify.
- [x] 7.6 Write failing tests: `[llm.openrouter.provider]` is sent as `provider`, unknown keys are rejected, and `fallback_models` is sent as `models` with the primary first. Confirm red. Implement, then verify.
- [x] 7.7 Write failing tests: `session_id` and `user` come from `LlmCallContext` and are omitted without it. Confirm red. Implement, then verify.
- [x] 7.8 Switch `OpenRouterProvider` from `ChatCompletionsProvider` to the new client, delete the dead code, and verify the existing OpenRouter tests still pass.
- [x] 7.9 Rewrite the chat, routing, identity and errors sections of `docs/openrouter.md`.
- [x] 7.10 Write failing wiremock tests for OpenRouter native `tool_choice`, `parallel_tool_calls` and JSON schema, each adding `provider.require_parameters: true`, plus the `response-healing` plugin on native non-streaming structured calls. Confirm red. Implement, then verify.

## 8. Model catalog and capabilities

- [x] 8.1 Write failing wiremock tests: context window is the minimum over selectable endpoints and respects `only` or `order` without fallbacks; `vision`, tools, reasoning and file input are derived from the catalog. Confirm red.
- [x] 8.2 Implement `catalog.rs` with an `ArcSwap` snapshot, a background fetch, and a 5-second first-call wait. Add `arc-swap` to the workspace dependencies. Verify 6.1.
- [x] 8.3 Write failing tests: an unknown model fails each call without an upstream request; a catalog outage yields conservative capabilities and refreshes after the backoff (driven by a test clock). Confirm red. Implement, then verify.
- [x] 8.4 Document capability detection in `docs/openrouter.md`.
- [x] 8.5 Write a failing test that chat-option support (`tool_choice`, `parallel_tool_calls`, `structured_outputs`/`response_format`, image and audio output) comes from the catalog. Confirm red. Implement, then verify.

## 9. Reasoning

- [x] 9.1 Write failing tests: config parsing, `effort` combined with `max_tokens` rejected, `reasoning` sent, and an effort the catalog does not support fails with the supported list. Confirm red. Implement, then verify.
- [x] 9.2 Write failing tests: text and summary details stream as `Thinking`, encrypted details are hidden, `exclude` emits nothing, and `finish_reason: "length"` with empty content is an error. Confirm red. Implement, then verify.
- [x] 9.3 Write failing tests: tool-call `reasoning_details` are stored as `ProviderState`, sent back unchanged and in order, survive a history rebuild from storage, and are not sent when the state's provider differs. Confirm red. Implement, then verify.
- [x] 9.4 Document reasoning in `docs/openrouter.md`.

## 10. Prompt caching

- [x] 10.1 Write failing table tests for breakpoint placement on Anthropic, Google, Qwen and OpenAI, with caching disabled, and with `ttl = "1h"`. Confirm red. Implement, then verify.
- [x] 10.2 Write a failing test that cached and cache-write tokens reach meta. Confirm red. Implement, then verify.
- [x] 10.3 Document caching in `docs/openrouter.md`.

## 11. Web search

- [x] 11.1 Write failing tests: the plugin options are sent, `HostedTool::WebSearch` is reported only when enabled, and `search_context_size` with a non-native engine is rejected. Confirm red. Implement, then verify.
- [x] 11.2 Write failing tests for citation rendering: dedupe, order, missing titles, URLs already linked inline, and the streamed final chunk equal to the persisted answer. Confirm red. Implement, then verify.
- [x] 11.3 Write a failing runtime test that the local web-search tool is not offered when the provider reports hosted web search. Confirm red, or green if this is already covered, and fix any gap. Then document in `docs/openrouter.md`.

## 12. PDFs

- [x] 12.1 Write failing tests: a PDF becomes a `file` part with a data URI, and the default filename is `document.pdf`. Confirm red. Implement, then verify.
- [x] 12.2 Write failing tests for the `pdf_engine` plugin, the `cloudflare-ai` default for models without native file input, no plugin for native models, and coexistence with the web plugin. Confirm red. Implement, then verify.
- [x] 12.3 Document PDFs in `docs/openrouter.md`.

## 13. Embeddings

- [x] 13.1 Write failing tests: `embedding_model` is optional, and Ollama, OpenAI and OpenRouter each resolve their own default. Confirm red. Make the field `Option` and update the providers and `cmd_doctor`, then verify.
- [x] 13.2 Write failing wiremock tests for OpenRouter `embed` and `embed_batch`: one request with an `input` array, results reordered by `index`, a count mismatch is an error, batches over 2048 are split, and errors are mapped. Confirm red. Implement, then verify.
- [x] 13.3 Write a failing `org_context` test for `EmbeddingProviderKind::OpenRouter`, covering the default model, key reuse and routing keys. Confirm red. Implement, then verify.
- [x] 13.4 Document embeddings in `docs/openrouter.md` and `docs/operations/configuration.md`.

## 14. Audio pipeline

- [x] 14.1 Write failing tests for MIME recognition: the new types, and parameters stripped. Confirm red. Implement, then verify.
- [x] 14.2 Write failing tests, skipped when ffmpeg is absent, for `probe_duration`, `to_wav_16k_mono` (including audio extracted from a video container), `split` and `transcode` covering pcm to mp3, wav to ogg/opus, and mp3 to wav. Confirm red. Implement `AudioPipeline`, then verify.
- [x] 14.3 Write failing tests for `ConvertingTranscriber`: it passes through accepted formats, converts the others, and gives a clear error when ffmpeg is missing (injected through the ffmpeg path). Confirm red. Implement, move Deepgram onto it, delete the ad-hoc conversion, then verify.

## 15. OpenRouter transcription

- [ ] 15.1 Write failing wiremock tests: JSON body with `input_audio`, `language`, `prompt` and `verbose_json`; language normalized to BCP-47; duration reported. Confirm red. Implement `openrouter.rs`, then verify.
- [ ] 15.2 Write failing tests: a 400 for `verbose_json` falls back once to `json` and is remembered; 413 gives the size error; 429 honors `Retry-After`. Confirm red. Implement, then verify.
- [ ] 15.3 Write failing tests for chunking: a 180-second recording becomes 4 ordered requests whose texts are joined; one failed chunk fails the whole transcription; unknown duration is sent whole; config caps `max_chunk_secs` at 55. Confirm red. Implement, then verify.
- [ ] 15.4 Write failing tests for `build_provider(cfg, llm).await` with the OpenRouter variant, key reuse and the missing-key error. Confirm red. Add the variant, make the builder async, and update the `web-ui` and `interface-cli` call sites. Verify with `cargo test -p assistant-transcription`.
- [ ] 15.5 Update `docs/transcription.md` and `docs/openrouter.md`.

## 16. OpenRouter TTS

- [ ] 16.1 Write failing tests: format config parsing (mp3, wav, ogg, pcm; others rejected), and the MIME type always matches the delivered format. Confirm red. Implement, then verify.
- [ ] 16.2 Write failing wiremock tests: request body with `speed` and `instructions`; request voice overrides config; the native format is requested when the model supports it. Confirm red. Implement `openrouter_tts.rs`, then verify.
- [ ] 16.3 Write failing tests for format selection and transcoding: a PCM-only HeyGen model read at 44.1 kHz and returned as mp3; Gemini to ogg via wav; the `pcm_sample_rate` override; missing ffmpeg fails the build. Confirm red. Implement, then verify.
- [ ] 16.4 Write failing tests for catalog validation: unknown voice, speed out of range, unsupported `instructions`, a request-level voice rejected without an upstream call, and a catalog outage that skips validation. Confirm red. Implement, then verify.
- [ ] 16.5 Write failing tests: `voice_response` uses the configured format; `/api/messages/{id}/tts` returns the provider MIME type. Confirm red. Implement, make `build_tts_provider(cfg, llm)` async with the OpenRouter variant, and update the call site. Verify.
- [ ] 16.6 Update `docs/transcription.md` (TTS) and `docs/openrouter.md`.

## 17. Generated media on responses

- [ ] 17.1 Write failing tests: `GeneratedMedia` on `LlmResponseMeta` clones cheaply (`Bytes`), and the orchestrator stores final-answer media as attachments of the assistant message, with the IDs included in `TurnResult.attachment_ids`. Confirm red. Implement, then verify.
- [ ] 17.2 Write a failing adapter-level test that an audio attachment from response media takes the same `channel-audio-outbound` path as `voice_response` audio. Confirm red, or green if this is already covered, and fix any gap.

## 18. Image generation

- [ ] 18.1 Write failing tests for `[images]` config parsing, including key reuse and routing keys. Confirm red. Implement, then verify.
- [ ] 18.2 Write failing wiremock tests for the OpenRouter images provider: request shape with `b64_json`; `url` entries downloaded; `session_id` and `user` from `LlmCallContext`; 402 and 429 handling; cost recorded. Confirm red. Implement `images/`, then verify.
- [ ] 18.3 Write failing tests for `/images/models` validation: unknown model fails the build; enum, range and unlisted parameters are rejected without an upstream call; reference images are rejected for models without image input. Confirm red. Implement, then verify.
- [ ] 18.4 Write failing tests for the `image-generate` tool: parameter schema; `reference_attachment_ids` resolved to data URIs, non-images rejected, at most 16; attachments returned; fewer images than requested reported; the tool is absent when `[images]` is unset. Confirm red. Implement and register it in `space_runtime`, then verify.
- [ ] 18.5 Write `docs/images.md` and link it from `docs/openrouter.md`.

## 19. Chat image output

- [ ] 19.1 Write failing tests: `image_output` with a catalog-capable model sends `modalities` and `image_config`; a model without image output fails each call and logs the error at startup. Confirm red. Implement, then verify.
- [ ] 19.2 Write failing tests: `message.images` data URIs are decoded, HTTPS URLs are downloaded, and images from deltas and the final message are collected without duplicates into `GeneratedMedia`. Confirm red. Implement, then verify.
- [ ] 19.3 Document chat image output in `docs/openrouter.md`.

## 20. Voice-native chat (audio input and replies)

- [ ] 20.1 Write failing tests: the web-ui voice send path and the Signal, Matrix, Mattermost and Nextcloud voice handlers attach the recording as `ContentBlock::Audio`, with the transcript as text, or `[Voice message]` when `[transcription]` is absent (no longer dropped). Confirm red. Add the `Audio` variant, implement, then verify.
- [ ] 20.1a Write failing tests: the orchestrator keeps audio only for the current turn and only for providers with audio input support; `history.rs` drops `Audio` from past messages; the Anthropic, OpenAI, Moonshot and Ollama builders drop `Audio`. Confirm red. Implement, then verify.
- [ ] 20.1b Write failing tests for OpenRouter audio input: `audio_input` config parsing; an `input_audio` part with conversion to WAV for formats outside `formats`; the transcript part prefixed "Transcript (may contain errors):"; `max_secs` cutoff; a 400 on `input_audio` leads to one repeat without audio, then disabled. Confirm red. Implement, then verify.
- [ ] 20.2 Write failing tests: a voice turn with audio-capable provider support requests audio options; a non-voice turn does not. Confirm red. Implement in the orchestrator, then verify.
- [ ] 20.3 Write failing SSE fixture tests for the OpenRouter client: `delta.audio.data` concatenated, transcript used when there is no content, PCM read at `pcm_sample_rate` and transcoded to `[tts] format`, attached as `GeneratedMedia::Audio`. Confirm red. Implement, then verify.
- [ ] 20.4 Write failing tests: a 400 on the audio modality repeats the call once without audio and disables audio for the provider instance; TTS fallback synthesizes the clip; text only when `[tts]` is absent. Confirm red. Implement, then verify.
- [ ] 20.5 Document voice-native chat (audio input and replies) in `docs/openrouter.md` and `docs/transcription.md`.

## 21. Doctor and final docs

- [ ] 21.1 Write failing tests for doctor: every OpenRouter-backed feature is listed with its key source and no key material; one `/key` call is made per distinct key; valid, invalid (401) and unreachable outcomes are reported. Confirm red. Implement, then verify.
- [ ] 21.2 Final pass on `docs/openrouter.md`: a "one key for everything" example and a full configuration reference. Check every claim against the code.
- [ ] 21.3 Run `make lint && make format && make test` and `openspec validate openrouter-full-provider --strict`. All must be clean.
