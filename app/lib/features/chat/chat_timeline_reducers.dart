import 'package:assistant_ui/models.dart';

import '../../api/models/stream_event.dart';
import 'chat_history_mapping.dart';
import 'interaction_kinds.dart';
import 'interaction_mapping.dart';

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
    if (event.toolName == renderSurfaceToolName &&
        resolvedStatus == ToolCallStatus.ok) {
      // The surface is the visible result; drop the pending chip.
      msgs.removeAt(entryIdx);
      return;
    }
    final record = msgs[entryIdx].toolCalls.first;
    record.status = resolvedStatus;
    record.arguments = event.arguments;
    record.result = event.result;
    record.data = event.data;
    record.app = event.app;
    record.duration = DateTime.now().difference(record.startedAt);
    msgs[entryIdx].isStreaming = false;
  } else {
    if (event.toolName == renderSurfaceToolName &&
        resolvedStatus == ToolCallStatus.ok) {
      return;
    }
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

/// Insert or update a surface timeline entry from a live [SurfaceUpdateEvent].
///
/// The first event of a surface inserts its entry before the streaming
/// assistant placeholder. Later events (a status change, or a replayed frame)
/// update the same entry in place, keyed by `surfaceId`; an empty `messages`
/// list keeps the rendered surface.
void applySurfaceUpdateEvent(List<ChatMessage> msgs, SurfaceUpdateEvent event) {
  final status = SurfaceStatus.fromWire(event.status);
  final idx = msgs.indexWhere(
    (m) =>
        m.timelineType == TimelineEntryType.surface &&
        m.surface?.surfaceId == event.surfaceId,
  );
  if (idx != -1) {
    final prev = msgs[idx].surface!;
    final submitted = status == SurfaceStatus.submitted;
    msgs[idx] = msgs[idx].copyWith(
      status: submitted ? MessageStatus.ok : msgs[idx].status,
      surface: prev.copyWith(
        title: event.title.isEmpty ? prev.title : event.title,
        fallbackText: event.fallbackText.isEmpty
            ? prev.fallbackText
            : event.fallbackText,
        status: status,
        answerable: submitted ? false : prev.answerable,
        messages: event.messages.isEmpty ? prev.messages : event.messages,
      ),
    );
    return;
  }

  insertBeforeStreamingPlaceholder(
    msgs,
    ChatMessage(
      id: 'surface-${event.surfaceId}',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.surface,
      surface: SurfaceRecord(
        surfaceId: event.surfaceId,
        toolCallId: event.toolCallId,
        anchorMessageId: event.anchorMessageId,
        anchorCallIndex: event.anchorCallIndex,
        title: event.title,
        fallbackText: event.fallbackText,
        status: status,
        answerable: status == SurfaceStatus.active,
        messages: event.messages,
      ),
    ),
  );
}

/// Close every answerable surface in place: a user message follows its anchor,
/// so it is read-only from now on (design D12). The status stays `active`; the
/// frame reads "Closed".
///
/// A surface action's own turn passes its surface as [exceptSurfaceId]: that
/// surface is settled by the server's status frame, not closed by the turn.
void closeActiveSurfaces(List<ChatMessage> msgs, {String? exceptSurfaceId}) {
  for (var i = 0; i < msgs.length; i++) {
    final surface = msgs[i].surface;
    if (surface == null ||
        surface.surfaceId == exceptSurfaceId ||
        surface.status != SurfaceStatus.active ||
        !surface.answerable) {
      continue;
    }
    msgs[i] = msgs[i].copyWith(surface: surface.copyWith(answerable: false));
  }
}

/// Show or clear the pending state of the surface timeline entry [surfaceId].
///
/// A post-turn action marks its surface pending while the request waits in the
/// queue and runs; its status `surface_update` or the turn's outcome clears it.
void markSurfacePending(
  List<ChatMessage> msgs,
  String surfaceId, {
  required bool pending,
}) {
  final idx = msgs.indexWhere(
    (m) =>
        m.timelineType == TimelineEntryType.surface &&
        m.surface?.surfaceId == surfaceId,
  );
  if (idx == -1) return;
  final next = pending ? MessageStatus.sending : MessageStatus.ok;
  if (msgs[idx].status == next) return;
  msgs[idx] = msgs[idx].copyWith(status: next);
}

/// Settle the surface [surfaceId] after its post-turn action was rejected.
///
/// A 409 means another route already submitted it, so it reads submitted. A
/// 410 means a later user turn closed it. Any other status leaves it active;
/// only the pending marker is cleared so the user can try again.
void settleRejectedSurfaceAction(
  List<ChatMessage> msgs,
  String surfaceId,
  int? statusCode,
) {
  final idx = msgs.indexWhere(
    (m) =>
        m.timelineType == TimelineEntryType.surface &&
        m.surface?.surfaceId == surfaceId,
  );
  if (idx == -1) return;
  final prev = msgs[idx].surface!;
  switch (statusCode) {
    case 409:
      msgs[idx] = msgs[idx].copyWith(
        status: MessageStatus.ok,
        surface: prev.copyWith(
          status: SurfaceStatus.submitted,
          answerable: false,
        ),
      );
    case 410:
      msgs[idx] = msgs[idx].copyWith(
        status: MessageStatus.ok,
        surface: prev.copyWith(answerable: false),
      );
    default:
      markSurfacePending(msgs, surfaceId, pending: false);
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

// -- Interactions ------------------------------------------------------------

/// Every interaction on the timeline, on tool calls and on its own entries.
Iterable<InteractionRecord> interactionsOf(List<ChatMessage> msgs) sync* {
  for (final m in msgs) {
    final own = m.interaction;
    if (own != null) yield own;
    for (final call in m.toolCalls) {
      final attached = call.interaction;
      if (attached != null) yield attached;
    }
  }
}

/// The record of the interaction [id], or null when it is not on the timeline.
InteractionRecord? findInteraction(List<ChatMessage> msgs, String id) =>
    interactionsOf(msgs).where((r) => r.id == id).firstOrNull;

/// Replace the interaction [id] with [update] of itself, wherever it renders.
/// Does nothing when it is not on the timeline.
void updateInteraction(
  List<ChatMessage> msgs,
  String id,
  InteractionRecord Function(InteractionRecord record) update,
) {
  for (final m in msgs) {
    final own = m.interaction;
    if (own != null && own.id == id) m.interaction = update(own);
    for (final call in m.toolCalls) {
      final attached = call.interaction;
      if (attached != null && attached.id == id) {
        call.interaction = update(attached);
      }
    }
  }
}

/// The progress label of the first interaction the turn still waits on, or
/// null when it waits on none.
String? awaitingLabelOf(List<ChatMessage> msgs) {
  final open = interactionsOf(msgs).where((r) => r.isOpen).firstOrNull;
  return open == null ? null : interactionKindSpec(open).progressLabel;
}

/// Show a requested interaction: in the footer of the tool call with its
/// `tool_call_id`, else as an entry of its own before the reply placeholder.
void applyInteractionRequested(
  List<ChatMessage> msgs,
  InteractionRequestedEvent event,
) {
  if (findInteraction(msgs, event.interactionId) != null) return;
  final record = interactionRecordFromEvent(event);
  final callId = event.toolCallId;
  final callIdx = callId == null
      ? -1
      : msgs.lastIndexWhere(
          (m) =>
              m.timelineType == TimelineEntryType.toolCall &&
              m.toolCalls.isNotEmpty &&
              m.toolCalls.first.toolCallId == callId,
        );
  if (callIdx != -1) {
    msgs[callIdx].toolCalls.first.interaction = record;
    return;
  }
  insertBeforeStreamingPlaceholder(
    msgs,
    ChatMessage(
      id: 'interaction-${event.interactionId}',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.interaction,
    )..interaction = record,
  );
}

/// End an interaction: a receipt for an answer, otherwise expired or
/// cancelled.
void applyInteractionResolved(
  List<ChatMessage> msgs,
  InteractionResolvedEvent event,
) {
  updateInteraction(
    msgs,
    event.interactionId,
    (r) => switch (event.outcome) {
      InteractionOutcome.resolved => r.copyWith(
        status: InteractionStatus.resolved,
        decision: approvalDecisionFrom(event.response),
        resolvedAt: event.resolvedAt,
        ruleId: event.ruleId,
        answers: questionAnswersFrom(event.response),
        declined: declinedFrom(event.response),
      ),
      InteractionOutcome.expired => r.copyWith(
        status: InteractionStatus.expired,
        resolvedAt: event.resolvedAt,
      ),
      InteractionOutcome.cancelled => r.copyWith(
        status: InteractionStatus.cancelled,
        resolvedAt: event.resolvedAt,
      ),
    },
  );
}

/// Mark every interaction the turn still waits on as cancelled: the turn is
/// over, so nobody can answer it any more.
void cancelOpenInteractions(List<ChatMessage> msgs) {
  for (final record in interactionsOf(msgs).where((r) => r.isOpen).toList()) {
    updateInteraction(
      msgs,
      record.id,
      (r) => r.copyWith(status: InteractionStatus.cancelled),
    );
  }
}
