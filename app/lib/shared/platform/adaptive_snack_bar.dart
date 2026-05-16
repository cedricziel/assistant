import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Shows a platform-appropriate transient message.
///
/// - Apple touch platforms: a Cupertino-styled toast in the overlay that
///   auto-dismisses after [duration].
/// - Material / macOS: a standard Material [SnackBar] via
///   [ScaffoldMessenger].
///
/// Phase 3 may swap the iOS path to a UIKit-embedded equivalent from
/// `adaptive_platform_ui`; the surface here stays unchanged.
void showAdaptiveSnackBar(
  BuildContext context,
  String message, {
  Duration duration = const Duration(seconds: 2),
}) {
  if (isAppleTouch) {
    _showCupertinoToast(context, message, duration);
    return;
  }
  ScaffoldMessenger.of(
    context,
  ).showSnackBar(SnackBar(content: Text(message), duration: duration));
}

void _showCupertinoToast(
  BuildContext context,
  String message,
  Duration duration,
) {
  final overlay = Overlay.maybeOf(context);
  if (overlay == null) return;

  late OverlayEntry entry;
  entry = OverlayEntry(
    builder: (ctx) => Positioned(
      left: 16,
      right: 16,
      bottom: MediaQuery.of(ctx).padding.bottom + 24,
      child: IgnorePointer(
        child: Center(
          child: CupertinoPopupSurface(
            isSurfacePainted: true,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              child: Text(
                message,
                style: CupertinoTheme.of(ctx).textTheme.textStyle,
              ),
            ),
          ),
        ),
      ),
    ),
  );

  overlay.insert(entry);
  Timer(duration, () {
    if (entry.mounted) entry.remove();
  });
}
