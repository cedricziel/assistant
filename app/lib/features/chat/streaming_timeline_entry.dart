import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/models/interaction_answer.dart';
import 'chat_provider.dart';
import 'interaction_footer.dart';
import 'mcp_app_frame.dart';

/// The lifecycle state of [message] as a timeline entry: waiting on its owner
/// while an interaction is open, else stale, active or complete.
EntryState timelineEntryState(ChatMessage message) {
  final waiting =
      (message.interaction?.isOpen ?? false) ||
      message.toolCalls.any((call) => call.interaction?.isOpen ?? false);
  if (waiting) return EntryState.awaitingInput;
  if (message.isStale) return EntryState.stale;
  return message.isStreaming ? EntryState.active : EntryState.complete;
}

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
    this.onInteractionAnswer,
  });

  final ChatMessage message;
  final EntryState entryState;

  /// The conversation the entry belongs to; an MCP App routes its calls by it.
  final String? conversationId;

  /// Called with an interaction's id and the owner's answer to it.
  final void Function(String interactionId, InteractionAnswer answer)?
  onInteractionAnswer;

  @override
  Widget build(BuildContext context) {
    return switch (message.timelineType) {
      TimelineEntryType.toolCall => _buildToolCall(context),
      TimelineEntryType.thinking => _buildThinking(context),
      TimelineEntryType.subagent => _buildSubagent(context),
      TimelineEntryType.command => const SizedBox.shrink(),
      TimelineEntryType.interaction =>
        _interactionFooter(message.interaction) ?? const SizedBox.shrink(),
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
      footer: _toolCallFooter(record),
    );
  }

  /// The footer slot is visible while the entry is collapsed. It holds the
  /// call's approval card and its MCP App, in that order.
  Widget? _toolCallFooter(ToolCallRecord record) {
    final interaction = _interactionFooter(record.interaction);
    final app = record.app == null
        ? null
        : McpAppFrame(toolCall: record, conversationId: conversationId);
    if (interaction != null && app != null) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [interaction, app],
      );
    }
    return interaction ?? app;
  }

  Widget? _interactionFooter(InteractionRecord? interaction) {
    if (interaction == null) return null;
    return InteractionFooter(
      record: interaction,
      onAnswer: (answer) => onInteractionAnswer?.call(interaction.id, answer),
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
