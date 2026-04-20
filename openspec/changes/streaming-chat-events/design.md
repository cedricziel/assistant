# Design: Streaming Chat Events

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           STREAMING DATA FLOW                               │
│                                                                             │
│  ┌─────────────────────┐                                                   │
│  │  Anthropic API      │                                                   │
│  │  (SSE stream)       │                                                   │
│  └────────┬────────────┘                                                   │
│           │ thinking_delta, text_delta, content_block_*                     │
│           ▼                                                                 │
│  ┌─────────────────────┐     StreamChunk::Thinking("...")                  │
│  │  Provider           │──────────────────────────────────┐                │
│  │  chat_sse()         │     StreamChunk::Text("...")      │                │
│  └────────┬────────────┘──────────────────────────────────┼──┐             │
│           │                                               │  │             │
│           │ LlmResponse::ToolCalls { thinking, items }    │  │             │
│           │ LlmResponse::FinalAnswer(text)                │  │             │
│           ▼                                               ▼  ▼             │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │  Orchestrator                                                    │       │
│  │                                                                  │       │
│  │  StreamChunk::Text → OrchestratorEvent::Token                   │       │
│  │  StreamChunk::Thinking → OrchestratorEvent::Thinking            │       │
│  │  ToolCalls.thinking → OrchestratorEvent::Thinking (batch)       │       │
│  │  tool dispatch → Status, ToolResult                             │       │
│  │  subagent spawn → SubagentEvent { agent_id, inner: ... }       │       │
│  │                                                                  │       │
│  └───────────────────────────┬─────────────────────────────────────┘       │
│                              │ mpsc::Sender<OrchestratorEvent>              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │  Web UI (SSE serializer)                                         │       │
│  │                                                                  │       │
│  │  Token → event: token, data: "..."                              │       │
│  │  Thinking → event: thinking, data: {"content":"..."}            │       │
│  │  SubagentEvent{id, Token} → event: subagent_token,              │       │
│  │                              data: {"agent_id":"..","token":".."}│       │
│  │  SubagentEvent{id, Thinking} → event: subagent_thinking, ...    │       │
│  │  SubagentEvent{id, ToolResult} → event: subagent_tool_result,.. │       │
│  │  SubagentEvent{id, Status} → event: subagent_status, ...        │       │
│  │                                                                  │       │
│  └───────────────────────────┬─────────────────────────────────────┘       │
│                              │ SSE (text/event-stream)                      │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────┐       │
│  │  Flutter App                                                     │       │
│  │                                                                  │       │
│  │  thinking → ThinkingEvent (token-level, accumulated)            │       │
│  │  subagent_token → SubagentTokenEvent                            │       │
│  │  subagent_thinking → SubagentThinkingEvent                      │       │
│  │  subagent_tool_result → SubagentToolResultEvent                 │       │
│  │  subagent_status → SubagentStatusEvent                          │       │
│  │                                                                  │       │
│  └─────────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Key Type Changes

### StreamChunk (crates/llm)

```rust
/// Typed chunk emitted during LLM streaming.
/// Replaces the raw `String` token sink.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Incremental text token (part of the final answer).
    Text(String),
    /// Incremental thinking/reasoning token.
    Thinking(String),
}
```

The provider trait method signature changes:

```rust
// Before:
async fn chat_streaming(
    &self, system: &str, history: &[ChatHistoryMessage],
    tools: &[ToolSpec], token_sink: Option<mpsc::Sender<String>>,
) -> Result<LlmResponse>;

// After:
async fn chat_streaming(
    &self, system: &str, history: &[ChatHistoryMessage],
    tools: &[ToolSpec], chunk_sink: Option<mpsc::Sender<StreamChunk>>,
) -> Result<LlmResponse>;
```

### LlmResponse modification

```rust
pub enum LlmResponse {
    FinalAnswer(String, ResponseMeta),
    ToolCalls(Vec<ToolCallItem>, ResponseMeta),  // no change to variant
    Thinking(String, ResponseMeta),
}

// Add thinking field to ToolCallItem's parent context:
// Option: Add a wrapper struct
pub struct ToolCallResponse {
    pub items: Vec<ToolCallItem>,
    pub thinking: Option<String>,  // thinking that preceded the tool calls
    pub meta: ResponseMeta,
}

// Then: LlmResponse::ToolCalls(ToolCallResponse)
// This avoids a breaking 3-field tuple and is cleaner.
```

**Decision**: Wrap `ToolCalls` in a struct to carry thinking alongside tool calls without changing the enum shape dramatically.

### OrchestratorEvent extension

```rust
pub enum OrchestratorEvent {
    Token(String),
    Status(String),
    ToolResult { tool_name, status, arguments, result },
    SkillComplete { skill_name, success, summary },
    AgentError { message },
    Thinking(String),
    SubagentStarted { agent_id, task },
    SubagentCompleted { agent_id, status, summary },
    AudioReady { audio_id },

    // NEW: wraps any inner event with subagent context
    SubagentEvent {
        agent_id: String,
        inner: Box<OrchestratorEvent>,
    },
}
```

## Orchestrator Adapter Logic

The orchestrator's forwarding task changes from:

```rust
// Before: simple String → Token wrapper
let forward_handle = tokio::spawn(async move {
    while let Some(t) = str_rx.recv().await {
        let _ = oe_sink.send(OrchestratorEvent::Token(t)).await;
    }
});
```

To:

```rust
// After: typed StreamChunk → OrchestratorEvent mapping
let forward_handle = tokio::spawn(async move {
    while let Some(chunk) = chunk_rx.recv().await {
        let event = match chunk {
            StreamChunk::Text(t) => OrchestratorEvent::Token(t),
            StreamChunk::Thinking(t) => OrchestratorEvent::Thinking(t),
        };
        let _ = oe_sink.send(event).await;
    }
});
```

After `chat_streaming` returns `ToolCalls`, emit any batch-accumulated thinking:

```rust
LlmResponse::ToolCalls(response) => {
    // Emit thinking that accompanied the tool calls
    if let Some(ref thinking) = response.thinking {
        if let Some(ref sink) = token_sink {
            let _ = sink.send(OrchestratorEvent::Thinking(thinking.clone())).await;
        }
    }
    // Process tool calls as before...
}
```

**Note**: With delta-level thinking streaming, the thinking will have already been sent token-by-token via `StreamChunk::Thinking`. The batch emit is a safety net for cases where the provider doesn't support delta streaming (OpenAI, Ollama). To avoid double-emitting for Anthropic, the provider should NOT populate `response.thinking` when it already streamed deltas. A `thinking_streamed: bool` flag or simply leaving `thinking: None` when deltas were sent handles this.

## Subagent Inner Event Forwarding

### Child Sink Creation

```rust
// In run_subagent(), before the loop:
let child_sink: Option<mpsc::Sender<OrchestratorEvent>> = if let Some(parent_conv_id) = spawn.parent_conversation_id {
    let sinks = self.token_sinks.read().await;
    if let Some(parent_sink) = sinks.get(&parent_conv_id) {
        let parent_sink = parent_sink.clone();
        let agent_id = spawn.agent_id.clone();
        let (child_tx, mut child_rx) = mpsc::channel::<OrchestratorEvent>(64);

        // Forward child events wrapped in SubagentEvent
        tokio::spawn(async move {
            while let Some(event) = child_rx.recv().await {
                let wrapped = OrchestratorEvent::SubagentEvent {
                    agent_id: agent_id.clone(),
                    inner: Box::new(event),
                };
                if parent_sink.send(wrapped).await.is_err() {
                    break;
                }
            }
        });

        Some(child_tx)
    } else {
        None
    }
} else {
    None
};
```

### Subagent Loop Changes

```rust
// Switch from .chat() to .chat_streaming()
let response = if let Some(ref sink) = child_sink {
    let (chunk_tx, mut chunk_rx) = mpsc::channel::<StreamChunk>(64);
    let sink_clone = sink.clone();
    let forward = tokio::spawn(async move {
        while let Some(chunk) = chunk_rx.recv().await {
            let event = match chunk {
                StreamChunk::Text(t) => OrchestratorEvent::Token(t),
                StreamChunk::Thinking(t) => OrchestratorEvent::Thinking(t),
            };
            let _ = sink_clone.send(event).await;
        }
    });
    let result = self.llm.chat_streaming(&system_prompt, &history, &tool_specs, Some(chunk_tx)).await;
    forward.await.ok();
    result
} else {
    self.llm.chat(&system_prompt, &history, &tool_specs).await
};

// Pass child_sink to finalize_tool_result (instead of None)
self.finalize_tool_result(
    &name,
    Some(&params),
    exec_result,
    elapsed,
    &mut otel_span,
    &mut history,
    &conv_store,
    conversation_id,
    turn_index,
    &mut scratch_attachments,
    &mut scratch_attachment_ids,
    child_sink.as_ref(),  // was: None
).await;
```

## Flutter UI Changes

### Dots Gate Fix (chat_screen.dart:651)

```dart
// Before:
child: message.isStreaming && message.content.isEmpty
    ? _streamingDotsIndicator()

// After:
child: message.isStreaming &&
       message.content.isEmpty &&
       message.toolCalls.isEmpty &&
       message.tokenStream == null
    ? _streamingDotsIndicator()
```

### New Stream Events (api_client.dart)

```dart
sealed class StreamEvent {}
// ... existing events ...

// NEW:
class SubagentTokenEvent extends StreamEvent {
  final String agentId;
  final String token;
}
class SubagentThinkingEvent extends StreamEvent {
  final String agentId;
  final String content;
}
class SubagentToolResultEvent extends StreamEvent {
  final String agentId;
  final String toolName;
  final String status;
  final String? arguments;
  final String? result;
}
class SubagentStatusEvent extends StreamEvent {
  final String agentId;
  final String message;
}
```

### Thinking: Token-Level Accumulation (chat_provider.dart)

Currently `ThinkingEvent` arrives as a full block. With delta streaming, multiple `ThinkingEvent`s arrive rapidly (one per token). The handler already accumulates:

```dart
void _onThinkingEvent(ChatState chatState, ThinkingEvent event) {
  // Existing logic already appends to thinkingContent — works as-is!
  msgs[existing] = prev.copyWith(
    thinkingContent: (prev.thinkingContent ?? '') + event.content,
  );
}
```

For real-time rendering, the thinking timeline entry needs its own `StreamController` (like the main message's `tokenStream`):

```dart
// Create a broadcast stream for thinking tokens
final thinkingController = StreamController<String>.broadcast();

// In _onThinkingEvent:
thinkingController.add(event.content);

// The timeline_section.dart renders with StreamMarkdown
```

### Subagent Inner Events (chat_provider.dart)

```dart
} else if (event is SubagentTokenEvent) {
  _onSubagentTokenEvent(chatState, event);
} else if (event is SubagentThinkingEvent) {
  _onSubagentThinkingEvent(chatState, event);
} else if (event is SubagentToolResultEvent) {
  _onSubagentToolResultEvent(chatState, event);
} else if (event is SubagentStatusEvent) {
  _onSubagentStatusEvent(chatState, event);
}
```

Each updates the corresponding subagent timeline entry's inner state (accumulated tokens, thinking, tool call list).

### Adaptive Timeline Widget Architecture

The existing `ChatTimelineSection` is replaced by `StreamingTimelineEntry` — a single adaptive widget that handles all timeline entry types and transitions between lifecycle states with appropriate visual weight.

#### Core Types

```dart
/// Density tier derived from screen width at the chat screen level.
enum TimelineDensity {
  compact,   // < 400px (phone)
  normal,    // 400–700px (tablet)
  expanded,  // > 700px (desktop)
}

/// Lifecycle state of a timeline entry. Drives visual treatment.
enum EntryState {
  active,    // currently running — auto-expanded, full visual weight
  complete,  // finished — auto-collapses after 500ms, normal opacity
  stale,     // from a previous turn — compressed, reduced opacity (0.6)
}

/// Focus position relative to the latest activity.
enum TimelineFocus {
  current,   // in the current agentic turn
  previous,  // from an earlier turn (triggers stale treatment)
}
```

#### Widget Structure

```dart
class StreamingTimelineEntry extends ConsumerStatefulWidget {
  const StreamingTimelineEntry({
    super.key,
    required this.message,
    required this.density,
    required this.focus,
  });

  final ChatMessage message;
  final TimelineDensity density;
  final TimelineFocus focus;
}

class _StreamingTimelineEntryState extends ConsumerState<StreamingTimelineEntry> {
  bool _expanded = false;
  bool _userPinned = false;  // user manually toggled — overrides auto-behavior
  Timer? _collapseTimer;

  EntryState get _effectiveState {
    if (widget.focus == TimelineFocus.previous) return EntryState.stale;
    return widget.message.entryState;
  }

  @override
  Widget build(BuildContext context) {
    final state = _effectiveState;

    // Auto-expand/collapse logic (respects user pin)
    if (!_userPinned) {
      if (state == EntryState.active && !_expanded) {
        _expanded = _shouldAutoExpand();
      } else if (state != EntryState.active && _expanded) {
        _scheduleCollapse();
      }
    }

    return AnimatedOpacity(
      opacity: state == EntryState.stale ? 0.6 : 1.0,
      duration: const Duration(milliseconds: 200),
      child: AnimatedSize(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeInOut,
        alignment: Alignment.topCenter,
        child: _buildForType(context, state),
      ),
    );
  }

  bool _shouldAutoExpand() {
    // Compact density: never auto-expand (tap to reveal)
    if (widget.density == TimelineDensity.compact) return false;
    return true;
  }

  void _scheduleCollapse() {
    _collapseTimer?.cancel();
    _collapseTimer = Timer(const Duration(milliseconds: 500), () {
      if (mounted && !_userPinned) {
        setState(() => _expanded = false);
      }
    });
  }
}
```

#### Density-Driven Rendering

```dart
Widget _buildForType(BuildContext context, EntryState state) {
  return switch (widget.message.timelineType) {
    TimelineEntryType.thinking => _buildThinking(context, state),
    TimelineEntryType.toolCall => _buildToolCall(context, state),
    TimelineEntryType.subagent => _buildSubagent(context, state),
    _ => const SizedBox.shrink(),
  };
}
```

#### Thinking Entry — State-Driven Layout

```dart
Widget _buildThinking(BuildContext context, EntryState state) {
  return switch (state) {
    EntryState.active => _buildThinkingActive(context),
    EntryState.complete => _buildThinkingComplete(context),
    EntryState.stale => _buildThinkingStale(context),
  };
}

Widget _buildThinkingActive(BuildContext context) {
  final maxHeight = switch (widget.density) {
    TimelineDensity.compact => 120.0,
    TimelineDensity.normal => 150.0,
    TimelineDensity.expanded => 200.0,
  };

  return _buildSection(
    header: Row(children: [
      Icon(Icons.psychology_outlined, size: 14),
      SizedBox(width: 4),
      Text('Thinking...'),
      Spacer(),
      _LiveDurationTimer(startedAt: widget.message.startedAt),
    ]),
    expandedContent: _expanded
      ? _MaxHeightFadeContainer(
          maxHeight: maxHeight,
          child: SingleChildScrollView(
            // Auto-scroll to bottom during streaming
            controller: _scrollController,
            child: widget.message.thinkingTokenStream != null
              ? StreamMarkdown(stream: widget.message.thinkingTokenStream!)
              : SmoothMarkdown(data: widget.message.thinkingContent ?? ''),
          ),
        )
      : null,
  );
}

Widget _buildThinkingComplete(BuildContext context) {
  return _buildSection(
    header: Row(children: [
      Icon(Icons.psychology_outlined, size: 14),
      SizedBox(width: 4),
      Text('Thought for ${widget.message.durationText}'),
      Spacer(),
      Icon(_expanded ? Icons.expand_less : Icons.expand_more, size: 14),
    ]),
    expandedContent: _expanded
      ? SmoothMarkdown(data: widget.message.thinkingContent ?? '')
      : null,
    onTap: () => setState(() {
      _expanded = !_expanded;
      _userPinned = true;
    }),
  );
}

Widget _buildThinkingStale(BuildContext context) {
  // Maximally compressed — just icon + duration
  return GestureDetector(
    onTap: () => setState(() { _expanded = !_expanded; _userPinned = true; }),
    child: Padding(
      padding: EdgeInsets.symmetric(vertical: 2),
      child: Row(children: [
        Icon(Icons.psychology_outlined, size: 12, color: Colors.grey),
        SizedBox(width: 4),
        Text(widget.message.durationText ?? '',
          style: TextStyle(fontSize: 11, color: Colors.grey)),
        if (_expanded) ...[/* full content */],
      ]),
    ),
  );
}
```

#### Max-Height + Fade Container

```dart
/// Constrains child height and applies a gradient fade at the bottom
/// when content overflows. Auto-scrolls to bottom when [autoScroll] is true.
class _MaxHeightFadeContainer extends StatelessWidget {
  const _MaxHeightFadeContainer({
    required this.maxHeight,
    required this.child,
    this.fadeHeight = 20.0,
  });

  final double maxHeight;
  final Widget child;
  final double fadeHeight;

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: BoxConstraints(maxHeight: maxHeight),
      child: ShaderMask(
        shaderCallback: (bounds) => LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Colors.white, Colors.white, Colors.transparent],
          stops: [0.0, 1.0 - (fadeHeight / bounds.height), 1.0],
        ).createShader(bounds),
        blendMode: BlendMode.dstIn,
        child: child,
      ),
    );
  }
}
```

#### Tool Call Entry — Density Adaptation

```dart
Widget _buildToolCall(BuildContext context, EntryState state) {
  final record = widget.message.toolCalls.isNotEmpty
      ? widget.message.toolCalls.first : null;
  if (record == null) return const SizedBox.shrink();

  final showArgs = switch (widget.density) {
    TimelineDensity.compact => false,
    TimelineDensity.normal => state == EntryState.active,
    TimelineDensity.expanded => true,
  };

  // Auto-expand on error
  final autoExpand = record.status == ToolCallStatus.error;

  return _buildSection(
    header: Row(children: [
      Icon(Icons.build_outlined, size: 14),
      SizedBox(width: 4),
      Flexible(child: Text(record.toolName, overflow: TextOverflow.ellipsis)),
      SizedBox(width: 8),
      _toolStatusWidget(record.status),
      if (record.duration != null) ...[
        SizedBox(width: 4),
        Text('${record.duration!.inMilliseconds}ms',
          style: TextStyle(fontSize: 11, color: Colors.grey)),
      ],
    ]),
    expandedContent: (_expanded || autoExpand) && (showArgs || record.status == ToolCallStatus.error)
      ? _toolDetails(context, record)
      : null,
    onTap: () => setState(() { _expanded = !_expanded; _userPinned = true; }),
  );
}
```

#### Subagent Entry — Nested Timeline

```dart
Widget _buildSubagent(BuildContext context, EntryState state) {
  final innerMaxHeight = switch (widget.density) {
    TimelineDensity.compact => 100.0,
    TimelineDensity.normal => 120.0,
    TimelineDensity.expanded => 150.0,
  };

  return _buildSection(
    header: Row(children: [
      Icon(Icons.smart_toy_outlined, size: 14),
      SizedBox(width: 4),
      Flexible(child: Text(
        widget.message.subagentTask ?? widget.message.subagentId ?? 'Agent',
        overflow: TextOverflow.ellipsis,
      )),
      Spacer(),
      if (state == EntryState.active)
        SizedBox(width: 12, height: 12,
          child: CircularProgressIndicator.adaptive(strokeWidth: 1.5))
      else ...[
        _subagentStatusIcon(),
        if (widget.message.durationText != null) ...[
          SizedBox(width: 4),
          Text(widget.message.durationText!,
            style: TextStyle(fontSize: 11, color: Colors.grey)),
        ],
      ],
      if (!_expanded) ...[
        SizedBox(width: 4),
        Icon(Icons.expand_more, size: 14),
      ],
    ]),
    expandedContent: _expanded
      ? _MaxHeightFadeContainer(
          maxHeight: innerMaxHeight,
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                // Inner thinking
                if (widget.message.subagentThinkingStream != null ||
                    widget.message.subagentThinking != null)
                  StreamingTimelineEntry(
                    message: _syntheticThinkingMessage(),
                    density: widget.density,
                    focus: widget.focus,
                  ),
                // Inner tool calls
                ...widget.message.subagentToolCalls.map((tc) =>
                  StreamingTimelineEntry(
                    message: _syntheticToolCallMessage(tc),
                    density: widget.density,
                    focus: widget.focus,
                  ),
                ),
                // Inner streamed answer
                if (widget.message.subagentTokenStream != null)
                  StreamMarkdown(stream: widget.message.subagentTokenStream!),
              ],
            ),
          ),
        )
      : null,
    onTap: () => setState(() { _expanded = !_expanded; _userPinned = true; }),
  );
}
```

#### Focus Management (in chat_provider.dart)

```dart
/// Called when a new timeline entry becomes active.
/// Transitions previous active entries to complete.
void _updateFocus(List<ChatMessage> msgs) {
  final streamingIdx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
  if (streamingIdx == -1) return;

  // Find all timeline entries between last user message and streaming placeholder
  for (var i = 0; i < streamingIdx; i++) {
    final msg = msgs[i];
    if (msg.timelineType != TimelineEntryType.message &&
        msg.entryState == EntryState.active) {
      msgs[i] = msg.copyWith(entryState: EntryState.complete);
    }
  }
}

/// Called when final answer tokens begin streaming.
/// All timeline entries become stale.
void _markAllStale(List<ChatMessage> msgs) {
  for (var i = 0; i < msgs.length; i++) {
    final msg = msgs[i];
    if (msg.timelineType != TimelineEntryType.message &&
        msg.entryState != EntryState.stale) {
      msgs[i] = msg.copyWith(entryState: EntryState.stale);
    }
  }
}
```

#### Accessibility

```dart
// In StreamingTimelineEntry.build():
final reduceMotion = MediaQuery.of(context).disableAnimations;

// When reduced motion is enabled:
// - Skip 500ms collapse delay (instant transition)
// - AnimatedSize duration = Duration.zero
// - AnimatedOpacity duration = Duration.zero
// - No auto-scroll animation (jump to bottom)

return Semantics(
  label: _semanticLabel(state),  // e.g. "Thinking, completed in 4.7 seconds"
  expanded: _expanded,
  child: /* ... */,
);
```

## Durable Event Store Compatibility

All new events are persisted automatically — the existing pattern in `web-ui/src/api/mod.rs` persists every `OrchestratorEvent` to `ConversationEventStore::append_event()`. The new `SubagentEvent` variant is serialized as its unwrapped SSE type (e.g., `subagent_token`) with the agent_id in the payload JSON. Reconnecting clients replay all events in order, including subagent inner events.

## Migration / Backwards Compatibility

- The `StreamChunk` type replaces `String` in the internal provider API — no external contract change.
- New SSE event types (`subagent_token`, `subagent_thinking`, `subagent_tool_result`, `subagent_status`) are additive — older Flutter clients ignore unknown event types gracefully (the SSE parser skips unrecognized events).
- The `ToolCallResponse` struct wrapping replaces the bare `Vec<ToolCallItem>` in `LlmResponse::ToolCalls` — this is a workspace-internal breaking change that requires updating all match arms. Scope: ~10 locations across runtime, orchestrator tests, and provider tests.

## Performance Considerations

- Thinking tokens arrive at the same rate as text tokens (~50-100/sec for Anthropic). The mpsc channel (capacity 64) and SSE serialization add negligible overhead.
- Subagent event forwarding adds one extra `tokio::spawn` per subagent and one channel hop. With typical subagent lifetimes (5-30s) this is insignificant.
- The durable event store will persist more events per run (thinking deltas = many small rows). May want to batch thinking deltas into fewer, larger persist calls (e.g., flush every 500ms or 20 tokens).

## Testing Strategy

- **Provider unit tests**: Verify `StreamChunk::Thinking` is emitted for `thinking_delta` SSE events (wiremock).
- **Orchestrator integration tests**: Verify `OrchestratorEvent::Thinking` arrives before tool call processing. Verify `SubagentEvent` wrapping.
- **Web UI tests**: Verify new SSE event types serialize correctly and are persisted.
- **Flutter widget tests**: Verify dots gate fix shows tool chips immediately. Verify nested timeline renders subagent events.
