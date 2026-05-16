# Streaming Chat Events

## Why

The chat UI shows bouncing dots for the entire duration of an agent turn — often 10–30 seconds of silence while the LLM thinks, calls tools, and runs subagents. The backend already has a streaming infrastructure (`OrchestratorEvent` → SSE → Flutter `StreamEvent`), but three systemic gaps prevent it from delivering real-time feedback:

1. **Thinking is discarded during tool-call turns.** The Anthropic provider accumulates `thinking_delta` tokens into a buffer, but the `LlmResponse` enum is mutually exclusive — when tool calls are present, thinking is silently dropped. The orchestrator never sees it.

2. **Subagents are opaque.** `SubagentStarted` and `SubagentCompleted` events exist, but the subagent loop uses the non-streaming `.chat()` API and passes `None` as the event sink. Inner thinking, tokens, and tool results are invisible to the parent.

3. **The Flutter dots gate hides available data.** The UI checks `isStreaming && content.isEmpty` and renders dots instead of the message bubble — even when `toolCalls` has been populated by `StatusEvent`/`ToolResultEvent`. Tool chips, which are already wired up, never render during streaming.

The result: users see nothing until the final answer arrives.

## What Changes

### Provider: Typed stream chunks

Replace the `mpsc::Sender<String>` token sink in `LlmProvider::chat_streaming()` with a typed `mpsc::Sender<StreamChunk>` enum:

```rust
enum StreamChunk {
    Text(String),
    Thinking(String),
}
```

The Anthropic provider forwards `thinking_delta` tokens immediately via `StreamChunk::Thinking` instead of silently buffering them. The orchestrator adapter maps `StreamChunk::Thinking` → `OrchestratorEvent::Thinking`.

Additionally, `LlmResponse::ToolCalls` gains an optional `thinking: Option<String>` field so accumulated thinking survives alongside tool calls. The orchestrator emits `OrchestratorEvent::Thinking(thinking)` before processing tool calls when present.

### Orchestrator: Subagent event forwarding

The subagent loop switches from `.chat()` to `.chat_streaming()` with a child event sink. Inner events are wrapped with the subagent's `agent_id` and forwarded to the parent conversation's sink. New `OrchestratorEvent` variants carry the nesting context:

```rust
SubagentEvent {
    agent_id: String,
    inner: Box<OrchestratorEvent>,
}
```

The web-ui SSE layer unwraps these into namespaced SSE events (e.g., `subagent_token`, `subagent_thinking`, `subagent_tool_result`) with `agent_id` in the JSON payload.

### Flutter: Remove dots gate, render nested events

- Fix the dots condition to only show dots when truly waiting (no tool calls, no token stream).
- Handle new `SubagentTokenEvent`, `SubagentThinkingEvent`, `SubagentToolResultEvent` stream events.
- Render subagent inner activity inside the existing `ChatTimelineSection` as a collapsible nested timeline.

## Capabilities

### New Capabilities

- **streaming-thinking**: Thinking tokens stream to the UI in real-time, token by token, rendered in an expandable "Thinking..." section as the LLM reasons.
- **streaming-subagent-inner**: Subagent thinking, tokens, and tool calls are forwarded to the parent stream and rendered as nested timeline entries inside the subagent section.

### Modified Capabilities

- **streaming-tool-calls**: Tool call chips now render immediately when a tool call begins, no longer hidden behind the dots gate. Status updates (pending → ok/error) animate in real-time.
- **streaming-reconnect**: The durable event store already persists all emitted events. New event types (thinking deltas, subagent inner events) are persisted too, so late-joining clients see full history.

## Impact

### Rust crates

- `crates/llm/src/` — new `StreamChunk` enum; update `LlmProvider::chat_streaming()` signature
- `crates/provider-anthropic/src/provider.rs` — forward `thinking_delta` via `StreamChunk::Thinking`; carry thinking in `ToolCalls` response
- `crates/provider-openai/src/provider.rs` — adapt to new `StreamChunk` type (text only for now)
- `crates/provider-ollama/` + `crates/llm/src/client.rs` — adapt to new `StreamChunk` type
- `crates/runtime/src/orchestrator/mod.rs` — map `StreamChunk` variants to `OrchestratorEvent`; emit thinking before tool calls
- `crates/runtime/src/orchestrator/stream_event.rs` — add `SubagentEvent` variant
- `crates/runtime/src/orchestrator/subagent.rs` — switch to `chat_streaming()`, create child sink, forward inner events
- `crates/web-ui/src/api/mod.rs` — unwrap `SubagentEvent` into namespaced SSE events

### Flutter app

- `app/lib/api/api_client.dart` — parse new SSE event types (`subagent_token`, `subagent_thinking`, `subagent_tool_result`)
- `app/lib/features/chat/chat_provider.dart` — handle new events, update subagent timeline entries with inner state
- `app/lib/features/chat/chat_screen.dart` — fix dots gate condition
- `app/lib/features/chat/timeline_section.dart` — render nested subagent timeline (thinking, tool calls, tokens)

### OpenAPI

- `openapi.json` — document new SSE event types in the streaming endpoint schema

## Non-goals

- **Streaming tool call arguments progressively** — The LLM streams partial JSON for tool parameters, but rendering half-formed JSON is low value and high complexity. Tool chips show the name immediately; arguments appear on completion.
- **Deep subagent nesting UI** — The nested timeline supports one level of subagent. If a subagent spawns its own subagent, inner-inner events are flattened into the parent subagent's section rather than nesting further.
- **Thinking token-by-token for non-Anthropic providers** — OpenAI and Ollama don't expose thinking tokens today. The `StreamChunk::Thinking` path is wired but only Anthropic produces events.
- **Changing the `LlmResponse` enum to a struct** — The mutually-exclusive enum is deeply embedded. We add an optional `thinking` field to `ToolCalls` rather than redesigning the return type.
