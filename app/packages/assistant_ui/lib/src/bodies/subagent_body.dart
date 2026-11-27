import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../models/timeline_state.dart';
import '../scopes/thread_density_scope.dart';
import '../theme/assistant_ui_colors.dart';
import '../timeline/timeline_entry_shell.dart';
import 'tool_status_icon.dart';

/// One subagent run: its task as the label, a status icon, and a collapsible
/// body that shows the inner timeline while it runs and the [summary] once it
/// has finished.
///
/// Each inner tool call is one compact row of icon, name and [ToolStatusIcon]
/// at every density. A row has no arguments, no result and no expansion, and
/// never consults a `ToolResultRendererScope`: rich tool renderers do not
/// appear in subagent timelines. Normal density shows the last two calls;
/// expanded density shows all of them.
class SubagentBody extends StatelessWidget {
  const SubagentBody({
    super.key,
    required this.label,
    this.summary,
    this.thinking = '',
    this.toolCalls = const [],
    this.entryState = EntryState.complete,
  });

  /// The subagent's task, or its id when it has no task.
  final String label;

  /// The completion summary; non-null once the subagent has finished.
  final String? summary;

  /// The subagent's reasoning so far, shown as one clipped line while running.
  final String thinking;

  /// The tool calls the subagent has made, in order.
  final List<ToolCallRecord> toolCalls;

  final EntryState entryState;

  double _maxHeight(BuildContext context) =>
      switch (ThreadDensityScope.of(context)) {
        TimelineDensity.compact => 100,
        TimelineDensity.normal => 120,
        TimelineDensity.expanded => 150,
      };

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final summary = this.summary;
    final hasCompleted = summary != null;
    final isActive = entryState == EntryState.active;

    Widget? body;
    if (hasCompleted) {
      body = Text(
        summary,
        style: TextStyle(fontSize: 12, color: colorScheme.onSurfaceVariant),
      );
    } else if (isActive) {
      body = _innerTimeline(context);
    }

    // Cancelled (stale after completion) is amber, completed is green, and a
    // running subagent shows a spinner.
    final Widget status;
    if (hasCompleted && entryState == EntryState.stale) {
      status = const Icon(
        Icons.cancel_outlined,
        size: 14,
        color: AssistantUiColors.toolDenied,
      );
    } else if (hasCompleted) {
      status = const Icon(
        Icons.check_circle_outline,
        size: 14,
        color: AssistantUiColors.toolOk,
      );
    } else {
      status = ToolStatusIcon.spinner;
    }

    return TimelineEntryShell(
      icon: Icons.smart_toy_outlined,
      label: label,
      status: status,
      entryState: entryState,
      expandable: hasCompleted || isActive,
      body: body,
    );
  }

  Widget _innerTimeline(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final entries = <Widget>[];

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

    var calls = toolCalls;
    if (ThreadDensityScope.of(context) == TimelineDensity.normal &&
        calls.length > 2) {
      calls = calls.sublist(calls.length - 2);
    }
    for (final call in calls) {
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
                call.toolName,
                style: TextStyle(
                  fontSize: 11,
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 4),
              ToolStatusIcon(status: call.status),
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
      maxHeight: _maxHeight(context),
      isActive: true,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: entries,
      ),
    );
  }
}
