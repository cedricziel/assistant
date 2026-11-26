import 'package:assistant_api/assistant_api.dart' as api show BranchInfo;
import 'package:assistant_api/assistant_api.dart'
    hide BranchInfo, ServerCapabilities;
import 'package:assistant_ui/models.dart';

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
List<ChatMessage> chatMessagesFromHistory(Iterable<MessageSummary> source) {
  final messages = <ChatMessage>[];
  for (final m in source) {
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
    if (isAssistantToolOnly) continue;

    messages.add(
      ChatMessage(
        id: m.id,
        role: m.role,
        content: m.content,
        status: parseFinishReason(m.finishReason),
        ttsAvailable: m.ttsAvailable,
        parentId: m.parentId,
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
      ),
    );
  }
  return messages;
}

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
