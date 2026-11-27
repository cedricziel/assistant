import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/models/interaction_answer.dart';
import '../../api/models/surface_action.dart';
import 'a2ui/surface_entry.dart';
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

/// The app's adapter for one timeline entry: [TimelineEntryView] plus the
/// footer the package cannot build.
///
/// A tool call entry shows its approval card and MCP App below the chip; an
/// interaction entry with no tool call shows its card as the entry itself.
class ChatTimelineEntry extends StatelessWidget {
  const ChatTimelineEntry({
    super.key,
    required this.message,
    this.entryState = EntryState.complete,
    this.conversationId,
    this.onInteractionAnswer,
    this.onSurfaceAction,
  });

  final ChatMessage message;
  final EntryState entryState;

  /// The conversation the entry belongs to; an MCP App routes its calls by it.
  final String? conversationId;

  /// Called with an interaction's id and the owner's answer to it.
  final void Function(String interactionId, InteractionAnswer answer)?
  onInteractionAnswer;

  /// Called when the user triggers an A2UI action on a surface entry.
  final void Function(SurfaceRecord surface, SurfaceActionSubmission action)?
  onSurfaceAction;

  @override
  Widget build(BuildContext context) {
    if (message.timelineType == TimelineEntryType.surface) {
      final surface = message.surface;
      if (surface == null) return const SizedBox.shrink();
      return SurfaceEntry(
        surface: surface,
        entryState: entryState,
        pending: message.status == MessageStatus.sending,
        onAction: onSurfaceAction,
      );
    }
    return TimelineEntryView(
      message: message,
      entryState: entryState,
      footer: _footer(),
    );
  }

  /// The footer slot is visible while the entry is collapsed. It holds the
  /// call's approval card and its MCP App, in that order.
  Widget? _footer() {
    if (message.timelineType == TimelineEntryType.toolCall) {
      final record = message.toolCalls.isNotEmpty
          ? message.toolCalls.first
          : null;
      if (record == null) return null;

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
    return _interactionFooter(message.interaction);
  }

  Widget? _interactionFooter(InteractionRecord? interaction) {
    if (interaction == null) return null;
    return InteractionFooter(
      record: interaction,
      onAnswer: (answer) => onInteractionAnswer?.call(interaction.id, answer),
    );
  }
}
