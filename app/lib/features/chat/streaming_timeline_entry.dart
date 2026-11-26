import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_smooth_markdown/flutter_smooth_markdown.dart'
    hide ToolCallStatus;

import 'chat_provider.dart';
import '../../shared/theme/legacy_theme.dart';
import 'markdown_link_handler.dart';
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

  double _maxHeight(BuildContext context, bool isSubagent) {
    final density = ThreadDensityScope.of(context);
    if (isSubagent) {
      return switch (density) {
        TimelineDensity.compact => 100,
        TimelineDensity.normal => 120,
        TimelineDensity.expanded => 150,
      };
    }
    return switch (density) {
      TimelineDensity.compact => 120,
      TimelineDensity.normal => 150,
      TimelineDensity.expanded => 200,
    };
  }

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

  Widget _buildThinking(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final content = message.thinkingContent;
    final isActive = entryState == EntryState.active;
    final stream = message.thinkingTokenStream;
    final hasContent = content != null && content.isNotEmpty;

    // Build the expanded content based on state.
    Widget? expandedContent;
    if (isActive && stream != null) {
      // Live streaming thinking — render with StreamMarkdown.
      expandedContent = TimelineEntryShell.fadeContainer(
        maxHeight: _maxHeight(context, false),
        isActive: true,
        child: StreamMarkdown(
          stream: stream,
          styleSheet: MarkdownStyleSheet.fromTheme(legacyThemeOf(context))
              .copyWith(
                paragraphStyle: TextStyle(
                  fontSize: 12,
                  color: colorScheme.onSurfaceVariant,
                  fontStyle: FontStyle.italic,
                ),
              ),
          onTapLink: (url) => MarkdownLinkHandler(context: context).onTap(url),
        ),
      );
    } else if (hasContent) {
      // Accumulated thinking content (complete or active without stream).
      expandedContent = TimelineEntryShell.fadeContainer(
        maxHeight: _maxHeight(context, false),
        isActive: isActive,
        child: SelectableText(
          content,
          style: TextStyle(
            fontSize: 12,
            color: colorScheme.onSurfaceVariant,
            fontStyle: FontStyle.italic,
          ),
        ),
      );
    }

    final expandable = hasContent || (isActive && stream != null);

    return _buildSection(
      context,
      icon: Icons.psychology_outlined,
      label: isActive ? 'Thinking...' : 'Thought',
      statusWidget: isActive
          ? const SizedBox(
              width: 12,
              height: 12,
              child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
            )
          : null,
      expandable: expandable,
      expandedContent: expandedContent,
    );
  }

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

  Widget _buildSubagent(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final hasCompleted = message.subagentSummary != null;
    final isActive = entryState == EntryState.active;

    Widget? expandedContent;
    if (hasCompleted) {
      expandedContent = Text(
        message.subagentSummary!,
        style: TextStyle(fontSize: 12, color: colorScheme.onSurfaceVariant),
      );
    } else if (isActive) {
      // Show inner timeline of subagent events.
      expandedContent = _buildSubagentInnerTimeline(context);
    }

    // Determine status icon: cancelled (stale+completed) → amber,
    // completed → green, active → spinner.
    final Widget statusIcon;
    if (hasCompleted && entryState == EntryState.stale) {
      statusIcon = const Icon(
        Icons.cancel_outlined,
        size: 14,
        color: Colors.amber,
      );
    } else if (hasCompleted) {
      statusIcon = const Icon(
        Icons.check_circle_outline,
        size: 14,
        color: Colors.green,
      );
    } else {
      statusIcon = const SizedBox(
        width: 12,
        height: 12,
        child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
      );
    }

    return _buildSection(
      context,
      icon: Icons.smart_toy_outlined,
      label: message.subagentTask ?? message.subagentId ?? '',
      statusWidget: statusIcon,
      expandable: hasCompleted || isActive,
      expandedContent: expandedContent,
    );
  }

  Widget _buildSubagentInnerTimeline(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final msg = message;
    final entries = <Widget>[];

    // Show subagent thinking if present.
    final thinking = msg.subagentThinking;
    if (thinking.isNotEmpty) {
      entries.add(
        Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: Row(
            children: [
              Icon(
                Icons.psychology_outlined,
                size: 12,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 4),
              Expanded(
                child: Text(
                  thinking,
                  style: TextStyle(
                    fontSize: 11,
                    color: colorScheme.onSurfaceVariant,
                    fontStyle: FontStyle.italic,
                  ),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ],
          ),
        ),
      );
    }

    // Show subagent tool calls.
    // Density adaptation: normal shows last 2 entries, expanded shows all.
    var toolCalls = msg.subagentToolCalls;
    if (ThreadDensityScope.of(context) == TimelineDensity.normal &&
        toolCalls.length > 2) {
      toolCalls = toolCalls.sublist(toolCalls.length - 2);
    }

    for (final tc in toolCalls) {
      entries.add(
        Padding(
          padding: const EdgeInsets.only(bottom: 2),
          child: Row(
            children: [
              Icon(
                Icons.build_outlined,
                size: 12,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 4),
              Text(
                tc.toolName,
                style: TextStyle(
                  fontSize: 11,
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 4),
              ToolStatusIcon(status: tc.status),
            ],
          ),
        ),
      );
    }

    if (entries.isEmpty) {
      return Text(
        'Processing...',
        style: TextStyle(
          fontSize: 11,
          color: colorScheme.onSurfaceVariant,
          fontStyle: FontStyle.italic,
        ),
      );
    }

    return TimelineEntryShell.fadeContainer(
      maxHeight: _maxHeight(context, true),
      isActive: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: entries,
      ),
    );
  }

  // -- Shared layout ----------------------------------------------------------

  Widget _buildSection(
    BuildContext context, {
    required IconData icon,
    required String label,
    Widget? statusWidget,
    bool expandable = false,
    Widget? expandedContent,
    bool forceExpand = false,
    Widget? footer,
  }) {
    return TimelineEntryShell(
      icon: icon,
      label: label,
      status: statusWidget,
      entryState: entryState,
      expandable: expandable,
      forceExpand: forceExpand,
      body: expandedContent,
      footer: footer,
    );
  }
}
