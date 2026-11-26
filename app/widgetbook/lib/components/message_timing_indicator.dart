import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

Widget _framed(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

final messageTimingIndicatorComponent = WidgetbookComponent(
  name: 'MessageTimingIndicator',
  useCases: [
    WidgetbookUseCase(
      name: 'full',
      builder: (context) => _framed(
        const MessageTimingIndicator(
          timing: MessageTiming(
            timeToFirstToken: Duration(milliseconds: 800),
            total: Duration(milliseconds: 12400),
            outputTokens: 420,
            decode: Duration(seconds: 10),
          ),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'total only',
      builder: (context) => _framed(
        const MessageTimingIndicator(
          timing: MessageTiming(total: Duration(milliseconds: 65000)),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'focused',
      builder: (context) => _framed(
        const _FocusOnFirstFrame(
          child: MessageTimingIndicator(
            timing: MessageTiming(
              timeToFirstToken: Duration(milliseconds: 800),
              total: Duration(milliseconds: 12400),
              outputTokens: 420,
              decode: Duration(seconds: 10),
            ),
          ),
        ),
      ),
    ),
  ],
);

/// Moves keyboard focus onto its first focusable descendant after the first
/// frame, as a Tab press would.
class _FocusOnFirstFrame extends StatefulWidget {
  const _FocusOnFirstFrame({required this.child});

  final Widget child;

  @override
  State<_FocusOnFirstFrame> createState() => _FocusOnFirstFrameState();
}

class _FocusOnFirstFrameState extends State<_FocusOnFirstFrame> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) FocusScope.of(context).nextFocus();
    });
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
