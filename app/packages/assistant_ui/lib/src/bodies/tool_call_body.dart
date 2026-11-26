import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../models/timeline_state.dart';
import '../scopes/thread_density_scope.dart';
import '../scopes/tool_result_renderer_scope.dart';
import '../timeline/timeline_entry_shell.dart';
import '../tool_results/generic_tool_result_view.dart';
import 'tool_status_icon.dart';

/// One tool call timeline entry: a [TimelineEntryShell] headed by the tool
/// name, a status icon and a duration badge, with the details as its body.
///
/// The details come from the nearest [ToolResultRendererScope] resolver; when
/// it is absent or returns `null`, [GenericToolResultView] renders. An error
/// status expands the details on its own. The [footer] renders below the
/// collapsible region and stays visible while collapsed, for example an
/// approval card or an MCP App frame supplied by the app.
class ToolCallBody extends StatelessWidget {
  const ToolCallBody({
    super.key,
    required this.record,
    this.entryState = EntryState.complete,
    this.footer,
  });

  final ToolCallRecord record;
  final EntryState entryState;
  final Widget? footer;

  static String _formatDuration(Duration d) {
    if (d.inSeconds >= 60) {
      return '${d.inMinutes}m ${d.inSeconds % 60}s';
    }
    return '${(d.inMilliseconds / 1000).toStringAsFixed(1)}s';
  }

  Widget _status(BuildContext context) {
    // The bare spinner lets the shell hide it at compact density.
    final Widget icon = record.status == ToolCallStatus.pending
        ? ToolStatusIcon.spinner
        : ToolStatusIcon(status: record.status);
    final duration = record.duration;
    if (duration == null ||
        ThreadDensityScope.of(context) == TimelineDensity.compact) {
      return icon;
    }
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        icon,
        const SizedBox(width: 4),
        Text(
          _formatDuration(duration),
          style: TextStyle(
            fontSize: 10,
            color: Theme.of(context).colorScheme.onSurfaceVariant
                .withAlpha(180),
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final hasDetails = record.arguments != null || record.result != null;
    return TimelineEntryShell(
      icon: Icons.build_outlined,
      label: record.toolName,
      status: _status(context),
      entryState: entryState,
      expandable: hasDetails,
      forceExpand: record.status == ToolCallStatus.error,
      body: hasDetails
          ? ToolResultRendererScope.of(context)?.call(context, record) ??
                GenericToolResultView(record: record)
          : null,
      footer: footer,
    );
  }
}
