# Tasks: Gemini Provider

## Phase 1: Core Types & Config

- [ ] **1.1** Add `Gemini` variant to `LlmProviderKind` in `crates/core/src/types.rs`
- [ ] **1.2** Add `Gemini` variant to `EmbeddingProviderKind` in `crates/core/src/types.rs`
- [ ] **1.3** Add `GeminiOptions` struct to `crates/core/src/types.rs` (fields: `google_search: bool`, `max_tokens: Option<u32>`)
- [ ] **1.4** Add `gemini: GeminiOptions` field to `LlmConfig`
- [ ] **1.5** Add config deserialization test: `provider = "gemini"` parses to `LlmProviderKind::Gemini`
- [ ] **1.6** Add config deserialization test: `[llm.embeddings] provider = "gemini"` parses to `EmbeddingProviderKind::Gemini`

## Phase 2: Message Conversion

- [ ] **2.1** Create `crates/llm-provider/src/gemini/mod.rs` with `GeminiProvider` struct and `GeminiConfig`
- [ ] **2.2** Implement `build_gemini_contents()`: convert `ChatHistoryMessage[]` to Gemini `contents[]` format — handle `Text`, `MultimodalUser`, `AssistantToolCalls`, `ToolResult` variants
- [ ] **2.3** Implement function ID tracking: maintain `pending_ids` vec to map `ToolResult` back to `functionCall` IDs (same pattern as Moonshot)
- [ ] **2.4** Implement system prompt extraction: system prompt goes to `system_instruction` field, not in `contents[]`
- [ ] **2.5** Add unit test: `build_gemini_contents()` with text-only conversation
- [ ] **2.6** Add unit test: `build_gemini_contents()` with multimodal user message (text + image)
- [ ] **2.7** Add unit test: `build_gemini_contents()` with tool calls and tool results (verify function IDs preserved)
- [ ] **2.8** Add unit test: `build_gemini_contents()` with multi-turn tool calls (late result ID tracking)

## Phase 3: Tool Conversion & Response Parsing

- [ ] **3.1** Implement `build_gemini_tools()`: convert `ToolSpec[]` to `functionDeclarations[]` format
- [ ] **3.2** Implement `build_gemini_tools_with_search()`: inject `googleSearch` tool when `google_search` config is enabled
- [ ] **3.3** Implement `parse_gemini_response()`: extract text parts → `FinalAnswer`, functionCall parts → `ToolCalls`, usageMetadata → `LlmResponseMeta`
- [ ] **3.4** Add unit test: tool spec conversion (verify name, description, parameters mapping)
- [ ] **3.5** Add unit test: parse response with text-only answer
- [ ] **3.6** Add unit test: parse response with function calls (verify args are Value not string)
- [ ] **3.7** Add unit test: parse response with mixed text and function calls
- [ ] **3.8** Add unit test: parse error response (4xx/5xx)

## Phase 4: Non-Streaming Chat

- [ ] **4.1** Implement `GeminiProvider::from_llm_config()`: resolve `GEMINI_API_KEY`, set base URL default, build config
- [ ] **4.2** Implement `chat()`: build request body, POST to `generateContent`, parse response
- [ ] **4.3** Add wiremock test: non-streaming text answer round-trip
- [ ] **4.4** Add wiremock test: non-streaming tool call round-trip (verify function ID in response)
- [ ] **4.5** Add wiremock test: API error handling (400, 401, 500)

## Phase 5: Streaming Chat

- [ ] **5.1** Implement `chat_streaming()`: POST to `streamGenerateContent?alt=sse`, parse SSE events, emit `StreamChunk::Text` for text parts
- [ ] **5.2** Handle SSE format: each `data:` line is a complete `generateContent` response with incremental parts
- [ ] **5.3** Accumulate function calls across SSE chunks (partial functionCall may span chunks)
- [ ] **5.4** Add wiremock test: streaming text accumulation with `StreamChunk` verification
- [ ] **5.5** Add wiremock test: streaming with tool calls in response

## Phase 6: Embeddings

- [ ] **6.1** Implement `embed()` on `GeminiProvider`: POST to `embedContent`, extract `embedding.values`
- [ ] **6.2** Create `GeminiEmbedder` struct implementing `EmbeddingProvider` for standalone embedding use via `[llm.embeddings]`
- [ ] **6.3** Wire `GeminiEmbedder` into the embedding provider factory (same file that creates Voyage/OpenAI embedders)
- [ ] **6.4** Add wiremock test: embedding request and response parsing
- [ ] **6.5** Add test: `EmbeddingProviderKind::Gemini` creates embedder via factory

## Phase 7: Google Search Grounding

- [ ] **7.1** When `gemini.google_search = true`, include `{"googleSearch": {}}` in the tools array
- [ ] **7.2** Report `HostedTool::WebSearch` in `capabilities()` when Google Search is enabled (so runtime suppresses local web-search tool)
- [ ] **7.3** Add test: capabilities include `HostedTool::WebSearch` when google_search is true
- [ ] **7.4** Add test: capabilities exclude `HostedTool::WebSearch` when google_search is false
- [ ] **7.5** Add wiremock test: request includes `googleSearch` tool when enabled

## Phase 8: Provider Registration & Integration

- [ ] **8.1** Register `GeminiProvider` in `create_provider()` match in `crates/llm-provider/src/lib.rs`
- [ ] **8.2** Export from `crates/llm-provider/src/lib.rs`
- [ ] **8.3** Add `create_provider()` factory test for Gemini
- [ ] **8.4** Add ignored integration test: live call to Gemini API with function calling (requires `GEMINI_API_KEY`)
- [ ] **8.5** Verify full workspace builds: `cargo check --workspace`

## Phase 9: Documentation & Polish

- [ ] **9.1** Add Gemini section to `config.toml` example with commented-out configuration
- [ ] **9.2** Add Gemini setup guide to `docs/` — free tier API key, model selection, Google Search grounding, embedding config
- [ ] **9.3** Run `make lint && make format` — fix any warnings
- [ ] **9.4** Run full test suite: `make test`
