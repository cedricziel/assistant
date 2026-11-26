import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';
import '../theme/assistant_ui_colors.dart';

/// The status indicator of a tool call: a spinner while pending, otherwise a
/// distinct icon per outcome so the result never relies on colour alone.
class ToolStatusIcon extends StatelessWidget {
  const ToolStatusIcon({super.key, required this.status});

  final ToolCallStatus status;

  /// The pending spinner as a bare [SizedBox], which `TimelineEntryShell`
  /// hides at compact density.
  static const Widget spinner = SizedBox(
    width: 12,
    height: 12,
    child: CircularProgressIndicator.adaptive(strokeWidth: 1.5),
  );

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return switch (status) {
      ToolCallStatus.pending => spinner,
      ToolCallStatus.ok => const Icon(
        Icons.check_circle_outline,
        size: 14,
        color: AssistantUiColors.toolOk,
      ),
      ToolCallStatus.error => Icon(
        Icons.error_outline,
        size: 14,
        color: colorScheme.error,
      ),
      ToolCallStatus.denied => const Icon(
        Icons.block,
        size: 14,
        color: AssistantUiColors.toolDenied,
      ),
      ToolCallStatus.cancelled => Icon(
        Icons.block,
        size: 14,
        color: colorScheme.onSurfaceVariant.withAlpha(180),
        semanticLabel: 'Cancelled',
      ),
    };
  }
}
