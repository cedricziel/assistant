import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';

import 'chat_provider.dart';
import 'mcp_app_frame.dart';

/// A streaming-aware timeline entry for one thinking, tool call or subagent
/// step in the chat timeline.
///
/// Builds a [TimelineEntryShell] around the entry's body; the shell owns
/// expansion, pinning, auto-collapse and stale opacity. Density comes from the
/// [ThreadDensityScope].
class StreamingTimelineEntry extends StatelessWidget {
  const StreamingTimelineEntry({
    super.key,
    required this.message,
    this.entryState = EntryState.complete,
    this.conversationId,
  });

  final ChatMessage message;
  final EntryState entryState;

  /// The conversation the entry belongs to; an MCP App routes its calls by it.
  final String? conversationId;

  @override
  Widget build(BuildContext context) {
    return switch (message.timelineType) {
      TimelineEntryType.toolCall => _buildToolCall(context),
      TimelineEntryType.thinking => _buildThinking(context),
      TimelineEntryType.subagent => _buildSubagent(context),
      TimelineEntryType.command => const SizedBox.shrink(),
      TimelineEntryType.interaction => const SizedBox.shrink(),
      TimelineEntryType.surface => const SizedBox.shrink(),
      TimelineEntryType.message => const SizedBox.shrink(),
    };
  }

  // -- Thinking ---------------------------------------------------------------

  Widget _buildThinking(BuildContext context) => ThinkingBody(
    content: message.thinkingContent,
    tokenStream: message.thinkingTokenStream,
    entryState: entryState,
  );

  // -- Tool call --------------------------------------------------------------

  Widget _buildToolCall(BuildContext context) {
    final record = message.toolCalls.isNotEmpty
        ? message.toolCalls.first
        : null;
    if (record == null) return const SizedBox.shrink();

    return ToolCallBody(
      record: record,
      entryState: entryState,
      // An MCP App renders in the footer slot: visible while collapsed.
      footer: record.app == null
          ? null
          : McpAppFrame(toolCall: record, conversationId: conversationId),
    );
  }

  // -- Subagent ---------------------------------------------------------------

  Widget _buildSubagent(BuildContext context) => SubagentBody(
    label: message.subagentTask ?? message.subagentId ?? '',
    summary: message.subagentSummary,
    thinking: message.subagentThinking,
    toolCalls: message.subagentToolCalls,
    entryState: entryState,
  );
}
