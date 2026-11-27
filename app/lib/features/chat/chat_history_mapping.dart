import 'package:assistant_api/assistant_api.dart'
    as api
    show BranchInfo, InteractionDetail;
import 'package:assistant_api/assistant_api.dart'
    hide BranchInfo, InteractionDetail, ServerCapabilities;
import 'package:assistant_ui/models.dart';

import 'interaction_mapping.dart';

/// Map a server tool-status string onto [ToolCallStatus]; unknown values
/// are treated as [ToolCallStatus.ok].
ToolCallStatus parseToolStatus(String status) {
  return switch (status) {
    'pending' => ToolCallStatus.pending,
    'ok' => ToolCallStatus.ok,
    'error' => ToolCallStatus.error,
    'denied' => ToolCallStatus.denied,
    'cancelled' => ToolCallStatus.cancelled,
    _ => ToolCallStatus.ok,
  };
}

/// Map a persisted feedback rating onto [MessageRating]; unknown values map
/// to `null` so a newer server cannot break history rendering.
MessageFeedback? _feedbackFromHistory(MessageFeedbackSummary? source) {
  if (source == null) return null;
  final rating = switch (source.rating) {
    'positive' => MessageRating.positive,
    'negative' => MessageRating.negative,
    _ => null,
  };
  if (rating == null) return null;
  return MessageFeedback(rating: rating, comment: source.comment);
}

/// Map a persisted timing summary onto [MessageTiming]; absent figures stay
/// absent.
MessageTiming? _timingFromHistory(MessageTimingSummary? source) =>
    source == null
    ? null
    : MessageTiming(
        timeToFirstToken: source.timeToFirstTokenMs == null
            ? null
            : Duration(milliseconds: source.timeToFirstTokenMs!),
        total: Duration(milliseconds: source.totalMs),
        outputTokens: source.outputTokens,
        decode: source.decodeMs == null
            ? null
            : Duration(milliseconds: source.decodeMs!),
      );

/// Map a persisted `finish_reason` onto [MessageStatus]: `"cancelled"` is a
/// reply the user stopped; anything else (including unknown values) is `ok`.
MessageStatus parseFinishReason(String? reason) {
  return switch (reason) {
    'cancelled' => MessageStatus.stopped,
    _ => MessageStatus.ok,
  };
}

/// Map a list of persisted [MessageSummary] history rows into the flat
/// [ChatMessage] list the UI renders.
///
/// Each persisted assistant row may carry tool calls. The OpenAI-style wire
/// format stores each tool invocation as its own assistant message with
/// `content == ""` and a single entry in `tool_calls` — this is required so
/// the subsequent `tool` role row can reference it via `tool_call_id`.
///
/// We split such a row into a standalone [TimelineEntryType.toolCall] chip
/// entry and *skip* the underlying empty message bubble: the chip already
/// fully represents that ReAct step. Without the skip, every tool-only turn
/// renders as an empty grey pill next to its chip.
///
/// Rows that carry user-visible content (or attachments) are always preserved
/// as a [TimelineEntryType.message] entry, even if they also have tool calls.
///
/// [interactions] are the conversation's stored interactions. Resolved ones
/// become receipts on the tool call they belong to; an interaction that names
/// a tool call not on the active branch renders nowhere.
List<ChatMessage> chatMessagesFromHistory(
  Iterable<MessageSummary> source, {
  Iterable<api.InteractionDetail> interactions = const [],
}) {
  final messages = <ChatMessage>[];
  final spans = <int, _TurnSpan>{};
  for (final m in source) {
    final span = spans.putIfAbsent(
      m.turn,
      () => _TurnSpan(m.turn, m.createdAt, messages.length),
    );
    if (m.createdAt.isBefore(span.first)) span.first = m.createdAt;
    if (m.createdAt.isAfter(span.last)) span.last = m.createdAt;

    final toolCalls = m.toolCalls?.toList() ?? const [];
    for (var i = 0; i < toolCalls.length; i++) {
      final tc = toolCalls[i];
      messages.add(
        ChatMessage(
          // Include the index so multiple invocations of the same tool on a
          // single assistant row produce distinct widget keys.
          id: 'toolcall-${tc.name}-${m.id}-$i',
          role: 'assistant',
          content: '',
          timelineType: TimelineEntryType.toolCall,
          toolCalls: [
            ToolCallRecord(
              toolName: tc.name,
              status: parseToolStatus(tc.status),
              arguments: tc.arguments?.asMap.cast<String, dynamic>(),
              result: tc.result,
              toolCallId: tc.toolCallId,
              data: tc.data?.asMap.cast<String, Object?>(),
              app: tc.app == null ? null : _mcpAppFromView(tc.app!),
            ),
          ],
        ),
      );
    }

    final attachments = m.attachments;
    final hasAttachments = attachments != null && attachments.isNotEmpty;
    final isAssistantToolOnly =
        m.role == 'assistant' &&
        m.content.isEmpty &&
        toolCalls.isNotEmpty &&
        !hasAttachments;
    if (!isAssistantToolOnly) {
      messages.add(
        ChatMessage(
          id: m.id,
          role: m.role,
          content: m.content,
          status: parseFinishReason(m.finishReason),
          ttsAvailable: m.ttsAvailable,
          parentId: m.parentId,
          timing: _timingFromHistory(m.timing),
          branch: _branchFromApi(m.branch),
          replyBranch: _branchFromApi(m.replyBranch),
          attachments: attachments
              ?.map(
                (a) => ChatAttachment(
                  id: a.id,
                  filename: a.filename,
                  mimeType: a.mimeType,
                  url: a.url,
                ),
              )
              .toList(),
        )..feedback = _feedbackFromHistory(m.feedback),
      );
    }
    span.end = messages.length;
  }
  _attachInteractions(messages, spans.values.toList(), interactions);
  return messages;
}

/// One persisted turn's index range in the rendered timeline.
class _TurnSpan {
  _TurnSpan(this.turn, this.first, this.start) : last = first, end = start;

  final int turn;
  DateTime first;
  DateTime last;
  final int start;
  int end;
}

/// Attach [interactions] to the tool calls they belong to, or to the end of
/// their turn when the request had no tool-call id.
///
/// An id that is not on the active branch is dropped: the receipt must not be
/// parked on an unrelated turn. Without an id the same per-turn ordinal join
/// as tool results applies (same tool name, in order), then the end of the
/// turn.
void _attachInteractions(
  List<ChatMessage> messages,
  List<_TurnSpan> spans,
  Iterable<api.InteractionDetail> interactions,
) {
  if (interactions.isEmpty || spans.isEmpty) return;
  final seen = <String>{};
  final pending = <InteractionRecord>[];
  for (final detail in interactions) {
    if (!seen.add(detail.interactionId)) continue;
    final record = interactionRecordFromDetail(detail);
    final id = detail.toolCallId;
    if (id != null) {
      final call = _callById(messages, id);
      if (call != null) {
        call.interaction = record;
      } else {
        // The call is on a branch this history does not render.
      }
      continue;
    }
    pending.add(record);
  }
  if (pending.isEmpty) return;

  final inserts = <int, List<ChatMessage>>{};
  for (final record in pending) {
    final span = _spanAt(spans, record.createdAt);
    final hint = _toolNameHint(record);
    final call = _freeCallIn(messages, span, hint);
    if (call != null) {
      call.interaction = record;
      continue;
    }
    inserts.putIfAbsent(span.end, () => []).add(_interactionEntry(record));
  }

  final indexes = inserts.keys.toList()..sort((a, b) => b.compareTo(a));
  for (final index in indexes) {
    messages.insertAll(index, inserts[index]!);
  }
}

ToolCallRecord? _callById(List<ChatMessage> messages, String id) {
  for (final message in messages) {
    for (final call in message.toolCalls) {
      if (call.toolCallId == id) return call;
    }
  }
  return null;
}

ToolCallRecord? _freeCallIn(
  List<ChatMessage> messages,
  _TurnSpan span,
  String? toolName,
) {
  for (var i = span.start; i < span.end && i < messages.length; i++) {
    final message = messages[i];
    if (message.timelineType != TimelineEntryType.toolCall) continue;
    for (final call in message.toolCalls) {
      if (call.interaction != null) continue;
      if (toolName == null || call.toolName == toolName) return call;
    }
  }
  return null;
}

/// The tool an interaction belongs to when the request carries no tool-call id.
String? _toolNameHint(InteractionRecord record) {
  if (record.toolName != null) return record.toolName;
  return record.kind == InteractionKind.question ? 'ask-user' : null;
}

_TurnSpan _spanAt(List<_TurnSpan> spans, DateTime at) {
  for (final span in spans) {
    if (!at.isBefore(span.first) && !at.isAfter(span.last)) return span;
  }
  _TurnSpan? prior;
  for (final span in spans) {
    if (!span.first.isAfter(at)) prior = span;
  }
  return prior ?? spans.first;
}

ChatMessage _interactionEntry(InteractionRecord record) => ChatMessage(
  id: 'interaction-${record.id}',
  role: 'assistant',
  content: '',
  timelineType: TimelineEntryType.interaction,
)..interaction = record;

BranchInfo? _branchFromApi(api.BranchInfo? info) => info == null
    ? null
    : BranchInfo(
        count: info.count,
        index: info.index,
        siblingIds: info.siblingIds.toList(),
      );

McpApp _mcpAppFromView(McpAppView view) => McpApp(
  server: view.server,
  resourceUri: view.resourceUri,
  html: view.html,
  csp: McpAppCsp(
    connectDomains: view.csp.connectDomains?.toList() ?? const [],
    resourceDomains: view.csp.resourceDomains?.toList() ?? const [],
    frameDomains: view.csp.frameDomains?.toList() ?? const [],
    baseUriDomains: view.csp.baseUriDomains?.toList() ?? const [],
  ),
  permissions: McpAppPermissions(
    camera: view.permissions.camera,
    microphone: view.permissions.microphone,
    geolocation: view.permissions.geolocation,
    clipboardWrite: view.permissions.clipboardWrite,
  ),
  prefersBorder: view.prefersBorder,
);
