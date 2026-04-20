import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'platform/platform.dart';

/// A visible error screen shown when an unhandled Flutter framework error
/// occurs during widget build, layout, or rendering.
///
/// On Apple touch platforms (iOS / iPadOS / Catalyst) this renders a
/// Cupertino-styled page; on Material platforms (web, Android, macOS desktop)
/// it uses Material 3 widgets — matching the project's adaptive pattern via
/// [isAppleTouch].
///
/// In debug/profile mode the full error details are shown so developers can
/// diagnose the issue. In release mode a friendlier message is displayed.
///
/// Wire this up via [ErrorWidget.builder] in `main()` so that broken widgets
/// render this screen instead of the default red/grey error box.
class ErrorScreen extends StatelessWidget {
  const ErrorScreen({super.key, required this.details});

  final FlutterErrorDetails details;

  @override
  Widget build(BuildContext context) {
    final content = _ErrorContent(details: details);

    if (isAppleTouch) {
      return CupertinoPageScaffold(
        navigationBar: const CupertinoNavigationBar(
          middle: Text('Something went wrong'),
          automaticBackgroundVisibility: false,
        ),
        child: content,
      );
    }

    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: content,
    );
  }
}

/// Shared body content used by both Cupertino and Material paths.
class _ErrorContent extends StatelessWidget {
  const _ErrorContent({required this.details});

  final FlutterErrorDetails details;

  @override
  Widget build(BuildContext context) {
    // Resolve colors via the Material theme — Cupertino path inherits a
    // Material Theme wrapper from AdaptiveApp, so this works on both.
    final colorScheme = Theme.of(context).colorScheme;

    return SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (!isAppleTouch) ...[
              Row(
                children: [
                  Icon(Icons.error_outline, color: colorScheme.error, size: 32),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      'Something went wrong',
                      style: Theme.of(context).textTheme.headlineSmall
                          ?.copyWith(
                            color: colorScheme.error,
                            fontWeight: FontWeight.w600,
                          ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 16),
            ],
            Text(
              _userMessage(details),
              style: Theme.of(
                context,
              ).textTheme.bodyLarge?.copyWith(color: colorScheme.onSurface),
            ),
            if (kDebugMode || kProfileMode) ...[
              const SizedBox(height: 24),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: colorScheme.errorContainer.withValues(alpha: 0.3),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: colorScheme.error.withValues(alpha: 0.3),
                  ),
                ),
                child: SelectableText(
                  details.exceptionAsString(),
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    fontFamily: 'monospace',
                    color: colorScheme.onErrorContainer,
                  ),
                ),
              ),
              if (details.stack != null) ...[
                const SizedBox(height: 16),
                _StackTraceSection(
                  stack: details.stack!,
                  colorScheme: colorScheme,
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }

  static String _userMessage(FlutterErrorDetails details) {
    if (kReleaseMode) {
      return 'An unexpected error occurred. '
          'Try refreshing the page or restarting the app.';
    }

    final buffer = StringBuffer();
    if (details.context != null) {
      buffer.writeln('Error caught by: ${details.context}');
    }
    if (details.library != null) {
      buffer.writeln('Library: ${details.library}');
    }
    return buffer.isEmpty
        ? details.exceptionAsString()
        : buffer.toString().trimRight();
  }
}

/// Expandable stack trace section.
///
/// Uses [ExpansionTile] on Material and a tap-to-expand pattern on Cupertino
/// (since ExpansionTile uses Material ink effects that look out of place on
/// iOS).
class _StackTraceSection extends StatefulWidget {
  const _StackTraceSection({required this.stack, required this.colorScheme});

  final StackTrace stack;
  final ColorScheme colorScheme;

  @override
  State<_StackTraceSection> createState() => _StackTraceSectionState();
}

class _StackTraceSectionState extends State<_StackTraceSection> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final traceBox = Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: widget.colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        widget.stack.toString(),
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          fontFamily: 'monospace',
          fontSize: 11,
          color: widget.colorScheme.onSurfaceVariant,
        ),
      ),
    );

    if (isAppleTouch) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CupertinoButton(
            padding: EdgeInsets.zero,
            onPressed: () => setState(() => _expanded = !_expanded),
            child: Row(
              children: [
                Icon(
                  _expanded
                      ? CupertinoIcons.chevron_down
                      : CupertinoIcons.chevron_right,
                  size: 16,
                  color: widget.colorScheme.onSurfaceVariant,
                ),
                const SizedBox(width: 8),
                Text(
                  'Stack trace',
                  style: Theme.of(context).textTheme.titleSmall,
                ),
              ],
            ),
          ),
          if (_expanded) ...[const SizedBox(height: 8), traceBox],
        ],
      );
    }

    return ExpansionTile(
      title: Text('Stack trace', style: Theme.of(context).textTheme.titleSmall),
      children: [traceBox],
    );
  }
}
