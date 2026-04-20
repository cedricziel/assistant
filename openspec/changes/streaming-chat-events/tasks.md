# Tasks: Streaming Chat Events

## Phase 1: Quick Win — Flutter Dots Gate Fix

- [x] **1.1** Fix dots gate condition in `chat_screen.dart:651` to only show dots when `content.isEmpty && toolCalls.isEmpty && tokenStream == null`
- [x] **1.2** Add widget test: assistant message with tool calls but empty content renders tool chips (not dots)
- [x] **1.3** Verify existing `StreamMarkdown` still renders correctly once content becomes non-empty

## Phase 2: Typed Stream Chunks + Thinking Deltas

- [x] **2.1** Define `StreamChunk` enum in `crates/llm/src/` (Text, Thinking variants)
- [x] **2.2** Update `LlmProvider::chat_streaming()` trait signature: `Option<mpsc::Sender<String>>` → `Option<mpsc::Sender<StreamChunk>>`
- [x] **2.3** Update Anthropic provider `chat_sse()`: emit `StreamChunk::Thinking` on `thinking_delta` events
- [x] **2.4** Update OpenAI provider: wrap text deltas in `StreamChunk::Text` (no thinking support)
- [x] **2.5** Update Ollama provider / LLM client: wrap tokens in `StreamChunk::Text`
- [x] **2.6** Update orchestrator forwarding task (`mod.rs:580-598`): map `StreamChunk` variants to `OrchestratorEvent`
- [x] **2.7** Add unit test: Anthropic provider emits `StreamChunk::Thinking` for thinking_delta (wiremock)
- [x] **2.8** Add integration test: orchestrator emits `OrchestratorEvent::Thinking` during streaming

## Phase 3: Carry Thinking with Tool Calls

- [x] **3.1** Define `ToolCallResponse` struct with `items: Vec<ToolCallItem>`, `thinking: Option<String>`, `meta: ResponseMeta`
- [x] **3.2** Change `LlmResponse::ToolCalls` to wrap `ToolCallResponse` instead of `(Vec<ToolCallItem>, ResponseMeta)`
- [x] **3.3** Update all `LlmResponse::ToolCalls` match arms across workspace (~10 locations)
- [x] **3.4** Anthropic provider: populate `thinking` field when `thinking_buf` is non-empty and tool blocks are present
- [x] **3.5** Anthropic provider: set `thinking: None` when deltas were already streamed (avoid double-emit)
- [x] **3.6** Orchestrator: emit `OrchestratorEvent::Thinking(thinking)` before processing tool calls when `response.thinking.is_some()`
- [x] **3.7** Add test: orchestrator emits Thinking event followed by Status+ToolResult events in correct order

## Phase 4: Subagent Inner Event Forwarding

- [x] **4.1** Add `SubagentEvent { agent_id: String, inner: Box<OrchestratorEvent> }` variant to `OrchestratorEvent`
- [x] **4.2** Create child sink in `run_subagent()` that wraps events in `SubagentEvent` and forwards to parent sink
- [x] **4.3** Switch subagent loop from `.chat()` to `.chat_streaming()` with child chunk sink
- [x] **4.4** Pass child event sink (instead of `None`) to `finalize_tool_result` in subagent loop
- [x] **4.5** Emit batch thinking from subagent tool-call responses (same as Phase 3 logic, using child sink)
- [x] **4.6** Web UI SSE serializer: unwrap `SubagentEvent` into namespaced SSE events (`subagent_token`, `subagent_thinking`, `subagent_tool_result`, `subagent_status`)
- [x] **4.7** Persist subagent events to durable event store with correct event_type
- [x] **4.8** Add integration test: subagent tool call produces `SubagentEvent{ToolResult}` on parent sink
- [x] **4.9** Add web-ui test: `SubagentEvent` serializes to correct SSE event type with agent_id in payload

## Phase 5: Adaptive Timeline Foundation

- [x] **5.1** Define `TimelineDensity` enum (`compact`, `normal`, `expanded`) derived from `MediaQuery.of(context).size.width` (<400, 400-700, >700)
- [x] **5.2** Define `EntryState` enum (`active`, `complete`, `stale`) on `ChatMessage` model — provider sets transitions on streaming events
- [x] **5.3** Create `StreamingTimelineEntry` widget that replaces `ChatTimelineSection` — accepts `message`, `density`, and `focus` (current vs previous turn)
- [x] **5.4** Implement state-driven expand/collapse: active entries auto-expand, complete entries auto-collapse (500ms delay via `Future.delayed` + `mounted` guard), stale entries stay collapsed with reduced opacity (0.6)
- [x] **5.5** Implement user-override pinning: manual tap to expand sets `userPinned: true`, overrides auto-collapse; manual collapse sets `userPinned: false`
- [x] **5.6** Implement max-height + fade for active thinking/subagent content: `ConstrainedBox(maxHeight)` + `ShaderMask` with `LinearGradient` fade at bottom 20px, `SingleChildScrollView` auto-scrolls to bottom during streaming
- [x] **5.7** Density-driven max-height: compact=120px, normal=150px, expanded=200px for thinking; compact=100px, normal=120px, expanded=150px for subagent inner timeline
- [x] **5.8** Add widget test: entry auto-collapses when state transitions from active to complete
- [x] **5.9** Add widget test: user pinned entry stays expanded despite state transition
- [x] **5.10** Add widget test: density changes based on MediaQuery width

## Phase 6: Adaptive Thinking Entry

- [x] **6.1** Update SSE parser to handle rapid `thinking` events (already accumulates — verify no performance issue with high-frequency rebuilds)
- [x] **6.2** Add `thinkingTokenStream: Stream<String>?` field to `ChatMessage` model
- [x] **6.3** Create `StreamController<String>.broadcast()` for thinking in `_streamMessage()`
- [x] **6.4** In `_onThinkingEvent`: feed tokens to thinking stream controller (in addition to accumulating content); set `entryState = active`
- [x] **6.5** Implement `_buildThinking` in `StreamingTimelineEntry`: active state shows `StreamMarkdown` inside max-height + fade container with live duration timer ("Thinking... 3.2s"); complete state shows single-line "Thought for 4.7s" with expand chevron; stale state shows compressed "💭 4.7s" at reduced opacity
- [x] **6.6** Density adaptation: compact always starts collapsed (tap to expand); normal/expanded auto-expand active entries
- [x] **6.7** Add widget test: thinking entry renders streaming markdown when active
- [x] **6.8** Add widget test: thinking entry collapses to duration summary when complete

## Phase 7: Adaptive Tool Call Entry

- [x] **7.1** Implement `_buildToolCall` in `StreamingTimelineEntry`: pending state shows spinner + tool name + arguments (desktop: inline args, phone: name only); complete/ok shows duration badge, collapses args; complete/error auto-expands to show error text in `colorScheme.error`
- [x] **7.2** Add duration tracking: record `startedAt` on `StatusEvent`, compute elapsed on `ToolResultEvent`, display as badge ("1.2s")
- [x] **7.3** Density adaptation: compact shows icon + name only; normal shows icon + name + status; expanded shows icon + name + args preview + status
- [x] **7.4** Migrate from `ToolCallChip` (existing) to `StreamingTimelineEntry` for tool calls — ensure `ToolCallChip` still works standalone for complete messages loaded from history
- [x] **7.5** Add widget test: error tool call auto-expands to show error details
- [x] **7.6** Add widget test: density compact renders tool name only

## Phase 8: Adaptive Subagent Entry + Inner Events

- [x] **8.1** Define new `StreamEvent` subclasses: `SubagentTokenEvent`, `SubagentThinkingEvent`, `SubagentToolResultEvent`, `SubagentStatusEvent`
- [x] **8.2** Update SSE parser `_parseSse()` to recognize `subagent_token`, `subagent_thinking`, `subagent_tool_result`, `subagent_status` event types
- [x] **8.3** Add `subagentThinking`, `subagentToolCalls`, `subagentTokenStream`, `subagentThinkingStream` fields to `ChatMessage` model
- [x] **8.4** Implement `_onSubagentTokenEvent`, `_onSubagentThinkingEvent`, `_onSubagentToolResultEvent`, `_onSubagentStatusEvent` handlers in chat_provider; set subagent entry `entryState = active` on first inner event
- [x] **8.5** Implement `_buildSubagent` in `StreamingTimelineEntry`: active state shows header + inner timeline (nested `StreamingTimelineEntry` instances for inner thinking/tool calls); complete state collapses to "🤖 researcher ✅ 8.3s — summary"; stale state shows compressed header at reduced opacity
- [x] **8.6** Inner timeline renders inside max-height + fade container (same ShaderMask pattern as thinking)
- [x] **8.7** Density adaptation: compact shows only header with spinner (tap to expand inner timeline); normal shows header + last 2 inner entries; expanded shows full inner timeline (scrollable)
- [x] **8.8** Add widget test: subagent timeline section renders nested tool call chips and thinking
- [x] **8.9** Add widget test: subagent collapses to summary line on completion

## Phase 9: Focus Management + Auto-Collapse Orchestration

- [x] **9.1** Implement focus tracking in `chat_provider.dart`: when a new timeline entry becomes active, set previous active entries to `complete` (triggers auto-collapse). When final answer tokens start streaming, set all timeline entries to `stale`.
- [x] **9.2** Add 500ms debounce before auto-collapse transition to avoid jarring flicker on rapid state changes (e.g., short thinking → immediate tool call)
- [x] **9.3** Ensure scroll position follows the latest active entry: `ScrollController.animateTo()` when new entry appears, but only if user hasn't manually scrolled up (respect `_atBottom` flag already in chat_screen.dart)
- [x] **9.4** Handle edge case: multiple concurrent tool calls — all show as active simultaneously, collapse together when all complete
- [x] **9.5** Add widget test: previous thinking entry collapses when new tool call becomes active
- [x] **9.6** Add widget test: all entries transition to stale when final answer streaming begins

## Phase 10: Polish & Edge Cases

- [x] **10.1** Throttle thinking delta persistence in durable event store (batch every 500ms or 20 tokens into one row)
- [x] **10.2** Verify reconnection (`GET /api/runs/{id}/events?since=N`) replays subagent events correctly and reconstructs entry states
- [x] **10.3** Verify `subagent_completed` event correctly finalizes nested timeline entry (stops spinners, triggers auto-collapse)
- [x] **10.4** Handle cancelled subagents gracefully (close child sink, mark timeline entry as cancelled with amber status)
- [x] **10.5** Ensure `AnimatedSize` transitions don't cause jank during rapid state changes — profile with Flutter DevTools
- [x] **10.6** Accessibility: ensure collapsed entries are announced with their summary by screen readers; auto-expanding does not steal VoiceOver/TalkBack focus
- [x] **10.7** Respect `MediaQuery.disableAnimations` — skip auto-collapse delay and AnimatedSize when reduced motion is enabled (instant transitions)
- [x] **10.8** Update `openapi.json` with new SSE event type documentation
- [x] **10.9** Run `make lint && make format && make test` — all green
- [x] **10.10** Run `make lint-flutter && make test-flutter` — all green
