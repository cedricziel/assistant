import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// A single action in an [showAdaptiveActionSheet] sheet.
///
/// [onPressed] receives the sheet's [BuildContext] so the callback can
/// `Navigator.of(ctx).pop(value)` to return a value to the awaiter.
class AdaptiveActionSheetAction<T> {
  const AdaptiveActionSheetAction({
    required this.label,
    required this.onPressed,
    this.isDestructive = false,
  });

  final String label;
  final void Function(BuildContext) onPressed;
  final bool isDestructive;
}

/// Shows a platform-appropriate action sheet.
///
/// - Apple touch platforms: [showCupertinoModalPopup] with
///   [CupertinoActionSheet].
/// - Material / macOS: [showModalBottomSheet] with a list of
///   [ListTile]s.
///
/// Returns the value popped by the chosen action, or `null` if dismissed.
Future<T?> showAdaptiveActionSheet<T>({
  required BuildContext context,
  String? title,
  String? message,
  required List<AdaptiveActionSheetAction<T>> actions,
  String cancelLabel = 'Cancel',
}) {
  if (isAppleTouch) {
    return showCupertinoModalPopup<T>(
      context: context,
      builder: (ctx) => CupertinoActionSheet(
        title: title != null ? Text(title) : null,
        message: message != null ? Text(message) : null,
        actions: actions
            .map(
              (a) => CupertinoActionSheetAction(
                onPressed: () => a.onPressed(ctx),
                isDestructiveAction: a.isDestructive,
                child: Text(a.label),
              ),
            )
            .toList(),
        cancelButton: CupertinoActionSheetAction(
          onPressed: () => Navigator.of(ctx).pop(),
          child: Text(cancelLabel),
        ),
      ),
    );
  }
  return showModalBottomSheet<T>(
    context: context,
    builder: (ctx) {
      final cs = Theme.of(ctx).colorScheme;
      return SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (title != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Text(title, style: Theme.of(ctx).textTheme.titleMedium),
              ),
            if (message != null)
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
                child: Text(message, style: Theme.of(ctx).textTheme.bodyMedium),
              ),
            ...actions.map(
              (a) => ListTile(
                title: Text(
                  a.label,
                  style: a.isDestructive ? TextStyle(color: cs.error) : null,
                ),
                onTap: () => a.onPressed(ctx),
              ),
            ),
            ListTile(
              title: Text(cancelLabel),
              onTap: () => Navigator.of(ctx).pop(),
            ),
          ],
        ),
      );
    },
  );
}
