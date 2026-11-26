import 'package:assistant_ui/models.dart';

import '../../api/models/stream_event.dart';
import 'chat_history_mapping.dart';

// Pure reducers that apply one SSE event to the chat message list.
//
// Each function mutates the [msgs] list it is given (callers pass a copy of
// `ChatState.messages`) and, like the notifier code they were extracted from,
// mutates the affected [ChatMessage] entries in place.

/// Extract a tool name from a [StatusEvent] message.
///
/// The server emits messages like "Calling tool: web-search". Strip the
/// prefix; fall back to the full string if the format doesn't match.
String extractToolName(String statusMessage) {
  const prefix = 'Calling tool: ';
  if (statusMessage.startsWith(prefix)) {
    return statusMessage.substring(prefix.length).trim();
  }
  return statusMessage;
}

/// Set the delivery [status] of the message with [id], if present.
void setMessageStatus(List<ChatMessage> msgs, String id, MessageStatus status) {
  final idx = msgs.indexWhere((m) => m.id == id);
  if (idx != -1) {
    msgs[idx] = msgs[idx].copyWith(status: status);
  }
}

/// Insert [entry] before the streaming assistant placeholder so timeline
/// entries render above the reply bubble; append when there is no placeholder.
void insertBeforeStreamingPlaceholder(
  List<ChatMessage> msgs,
  ChatMessage entry,
) {
  final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
  if (idx != -1) {
    msgs.insert(idx, entry);
  } else {
    msgs.add(entry);
  }
}

/// Mark all currently-active timeline entries (thinking, subagent) as
/// complete. Called when a new timeline entry starts, so the previous one
/// auto-collapses.
void completeActiveTimelineEntries(List<ChatMessage> msgs) {
  for (var i = 0; i < msgs.length; i++) {
    if (msgs[i].timelineType != TimelineEntryType.message &&
        msgs[i].isStreaming) {
      msgs[i].isStreaming = false;
    }
  }
}

/// Mark all timeline entries as stale — called when final answer tokens
/// begin streaming, causing all timeline entries to collapse with reduced
/// opacity.
void markTimelineEntriesStale(List<ChatMessage> msgs) {
  for (var i = 0; i < msgs.length; i++) {
    if (msgs[i].timelineType != TimelineEntryType.message) {
      msgs[i].isStreaming = false;
      msgs[i].isStale = true;
    }
  }
}

/// Mark thinking entries that are still streaming as complete.
void completeStreamingThinkingEntries(List<ChatMessage> msgs) {
  for (var i = 0; i < msgs.length; i++) {
    if (msgs[i].timelineType == TimelineEntryType.thinking &&
        msgs[i].isStreaming) {
      msgs[i] = msgs[i].copyWith(isStreaming: false);
    }
  }
}

/// Create a tool-call timeline entry for a [StatusEvent] and insert it before
/// the streaming assistant placeholder.
void applyStatusEvent(List<ChatMessage> msgs, StatusEvent event) {
  final toolName = extractToolName(event.message);

  // Complete any currently-active timeline entries before adding a new one.
  completeActiveTimelineEntries(msgs);

  final entryId = 'toolcall-$toolName-${DateTime.now().millisecondsSinceEpoch}';
  insertBeforeStreamingPlaceholder(
    msgs,
    ChatMessage(
      id: entryId,
      role: 'assistant',
      content: '',
      isStreaming: true,
      timelineType: TimelineEntryType.toolCall,
      toolCalls: [
        ToolCallRecord(
          toolName: toolName,
          status: ToolCallStatus.pending,
          toolCallId: event.toolCallId,
        ),
      ],
    ),
  );
}

/// Resolve the pending tool-call timeline entry for [event].
///
/// Matches by `toolCallId` when available (stable) and falls back to the tool
/// name. When no pending entry matches, a resolved entry is inserted directly.
void applyToolResultEvent(List<ChatMessage> msgs, ToolResultEvent event) {
  final resolvedStatus = parseToolStatus(event.status);

  final entryIdx = event.toolCallId != null
      ? msgs.lastIndexWhere(
          (m) =>
              m.timelineType == TimelineEntryType.toolCall &&
              m.toolCalls.isNotEmpty &&
              m.toolCalls.first.toolCallId == event.toolCallId &&
              m.toolCalls.first.status == ToolCallStatus.pending,
        )
      : msgs.lastIndexWhere(
          (m) =>
              m.timelineType == TimelineEntryType.toolCall &&
              m.toolCalls.isNotEmpty &&
              m.toolCalls.first.toolName == event.toolName &&
              m.toolCalls.first.status == ToolCallStatus.pending,
        );

  if (entryIdx != -1) {
    final record = msgs[entryIdx].toolCalls.first;
    record.status = resolvedStatus;
    record.arguments = event.arguments;
    record.result = event.result;
    record.data = event.data;
    record.app = event.app;
    record.duration = DateTime.now().difference(record.startedAt);
    msgs[entryIdx].isStreaming = false;
  } else {
    final entryId =
        'toolcall-${event.toolName}-${DateTime.now().millisecondsSinceEpoch}';
    insertBeforeStreamingPlaceholder(
      msgs,
      ChatMessage(
        id: entryId,
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(
            toolName: event.toolName,
            status: resolvedStatus,
            toolCallId: event.toolCallId,
            arguments: event.arguments,
            result: event.result,
            data: event.data,
            app: event.app,
          ),
        ],
      ),
    );
  }
}

/// Insert or update a thinking timeline entry.
///
/// Multiple [ThinkingEvent]s accumulate into a single streaming entry. A new
/// entry is wired to [tokenStream] for incremental rendering.
void applyThinkingEvent(
  List<ChatMessage> msgs,
  ThinkingEvent event, {
  Stream<String>? tokenStream,
}) {
  final existing = msgs.indexWhere(
    (m) => m.timelineType == TimelineEntryType.thinking && m.isStreaming,
  );
  if (existing != -1) {
    final prev = msgs[existing];
    msgs[existing] = prev.copyWith(
      thinkingContent: (prev.thinkingContent ?? '') + event.content,
    );
    return;
  }

  // Complete previous active timeline entries before starting a new one.
  completeActiveTimelineEntries(msgs);
  final entry = ChatMessage(
    id: 'thinking-${DateTime.now().millisecondsSinceEpoch}',
    role: 'assistant',
    content: '',
    isStreaming: true,
    timelineType: TimelineEntryType.thinking,
    thinkingContent: event.content,
  );
  entry.thinkingTokenStream = tokenStream;
  insertBeforeStreamingPlaceholder(msgs, entry);
}

/// Insert a subagent timeline entry when a subagent starts.
void applySubagentStartedEvent(
  List<ChatMessage> msgs,
  SubagentStartedEvent event,
) {
  insertBeforeStreamingPlaceholder(
    msgs,
    ChatMessage(
      id: 'subagent-${event.agentId}',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.subagent,
      subagentId: event.agentId,
      subagentTask: event.task,
    ),
  );
}

/// Update the matching subagent entry with its completion summary.
void applySubagentCompletedEvent(
  List<ChatMessage> msgs,
  SubagentCompletedEvent event,
) {
  final entry = _subagentEntry(msgs, event.agentId);
  if (entry == null) return;
  entry.subagentSummary = event.summary;
  entry.isStreaming = false;
  // Mark cancelled subagents as stale for amber/collapsed display.
  if (event.status == 'cancelled') {
    entry.isStale = true;
  }
}

/// Accumulate token content from a subagent into its timeline entry.
void applySubagentTokenEvent(List<ChatMessage> msgs, SubagentTokenEvent event) {
  _subagentEntry(msgs, event.agentId)?.subagentContent += event.content;
}

/// Accumulate thinking content from a subagent into its timeline entry.
void applySubagentThinkingEvent(
  List<ChatMessage> msgs,
  SubagentThinkingEvent event,
) {
  _subagentEntry(msgs, event.agentId)?.subagentThinking += event.content;
}

/// Record a tool call result from a subagent into its timeline entry.
void applySubagentToolResultEvent(
  List<ChatMessage> msgs,
  SubagentToolResultEvent event,
) {
  _subagentEntry(msgs, event.agentId)?.subagentToolCalls.add(
    ToolCallRecord(
      toolName: event.toolName,
      status: parseToolStatus(event.status),
      arguments: event.arguments,
      result: event.result,
    ),
  );
}

ChatMessage? _subagentEntry(List<ChatMessage> msgs, String agentId) {
  for (final m in msgs) {
    if (m.timelineType == TimelineEntryType.subagent &&
        m.subagentId == agentId) {
      return m;
    }
  }
  return null;
}
