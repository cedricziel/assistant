import 'package:material_ui/material_ui.dart';

import '../models/message_timing.dart';

/// Small speed glyph that reveals how long a reply took.
///
/// A focusable button: the tooltip with [formatMessageTiming] shows on mouse
/// hover, on keyboard focus and on activation (tap, Enter or Space), so no
/// pointer is needed. Screen readers get [describeMessageTiming] as the label
/// without a hover. The hit area is 24×24 logical pixels.
class MessageTimingIndicator extends StatefulWidget {
  const MessageTimingIndicator({super.key, required this.timing});

  final MessageTiming timing;

  @override
  State<MessageTimingIndicator> createState() => _MessageTimingIndicatorState();
}

class _MessageTimingIndicatorState extends State<MessageTimingIndicator> {
  final _tooltipKey = GlobalKey<TooltipState>();

  void _showTooltip() => _tooltipKey.currentState?.ensureTooltipVisible();

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.onSurfaceVariant;

    return Tooltip(
      key: _tooltipKey,
      message: formatMessageTiming(widget.timing),
      triggerMode: TooltipTriggerMode.manual,
      showDuration: const Duration(seconds: 4),
      // The label below carries the same facts; do not read them twice.
      excludeFromSemantics: true,
      child: Semantics(
        button: true,
        label: describeMessageTiming(widget.timing),
        child: InkWell(
          onTap: _showTooltip,
          onFocusChange: (focused) {
            if (focused) _showTooltip();
          },
          borderRadius: BorderRadius.circular(12),
          child: SizedBox.square(
            dimension: 24,
            child: ExcludeSemantics(
              child: Icon(Icons.speed_outlined, size: 16, color: color),
            ),
          ),
        ),
      ),
    );
  }
}
