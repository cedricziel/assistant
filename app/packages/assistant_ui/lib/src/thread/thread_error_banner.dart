import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// Inline banner above the thread showing a turn or connection error, with a
/// close button that calls [onDismiss].
class ThreadErrorBanner extends StatelessWidget {
  const ThreadErrorBanner({
    super.key,
    required this.message,
    required this.onDismiss,
  });

  final String message;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AssistantUiSpacing.lg,
          vertical: AssistantUiSpacing.sm,
        ),
        child: Row(
          children: [
            Icon(
              Icons.warning_amber_outlined,
              color: colorScheme.onErrorContainer,
              size: 18,
            ),
            const SizedBox(width: AssistantUiSpacing.sm),
            Expanded(
              child: Text(
                message,
                style: TextStyle(color: colorScheme.onErrorContainer),
              ),
            ),
            IconButton(
              icon: const Icon(Icons.close, size: 18),
              onPressed: onDismiss,
            ),
          ],
        ),
      ),
    );
  }
}
