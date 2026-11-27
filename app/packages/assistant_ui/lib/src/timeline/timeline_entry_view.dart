import 'package:material_ui/material_ui.dart';

import '../bodies/command_body.dart';
import '../bodies/subagent_body.dart';
import '../bodies/thinking_body.dart';
import '../bodies/tool_call_body.dart';
import '../models/chat_message.dart';
import '../models/timeline_state.dart';

/// A pure dispatcher from a [TimelineEntryType] to its body: the single entry
/// point the chat screen uses to render a thinking, tool call, subagent,
/// command or interaction entry.
///
/// The [footer] is forwarded to the shell of a tool call entry, so it stays
/// visible while the entry is collapsed. Interaction entries, which have no
/// shell, render it as their content. [entryState] reaches every shell.
class TimelineEntryView extends StatelessWidget {
  const TimelineEntryView({
    super.key,
    required this.message,
    this.entryState = EntryState.complete,
    this.footer,
  });

  final ChatMessage message;
  final EntryState entryState;

  /// Content below the collapsible region of a tool call entry, or the content
  /// of an interaction entry.
  final Widget? footer;

  @override
  Widget build(BuildContext context) {
    return switch (message.timelineType) {
      TimelineEntryType.toolCall => _buildToolCall(),
      TimelineEntryType.thinking => ThinkingBody(
        content: message.thinkingContent,
        tokenStream: message.thinkingTokenStream,
        entryState: entryState,
      ),
      TimelineEntryType.subagent => SubagentBody(
        label: message.subagentTask ?? message.subagentId ?? '',
        summary: message.subagentSummary,
        thinking: message.subagentThinking,
        toolCalls: message.subagentToolCalls,
        entryState: entryState,
      ),
      TimelineEntryType.command => CommandBody(
        name: message.commandName ?? '',
        ack: message.commandAckText ?? message.content,
      ),
      TimelineEntryType.interaction => footer ?? const SizedBox.shrink(),
      // Surfaces render inside the message bubble; a bare surface entry has
      // nothing of its own to show.
      TimelineEntryType.surface => const SizedBox.shrink(),
      TimelineEntryType.message => const SizedBox.shrink(),
    };
  }

  Widget _buildToolCall() {
    final record = message.toolCalls.isNotEmpty
        ? message.toolCalls.first
        : null;
    if (record == null) return const SizedBox.shrink();

    return ToolCallBody(record: record, entryState: entryState, footer: footer);
  }
}
