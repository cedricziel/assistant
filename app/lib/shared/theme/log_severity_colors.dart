import 'package:flutter/material.dart';

/// Background + foreground colour pair for a log-level chip.
typedef LogSeverityColors = ({Color bg, Color fg});

/// Returns the chip colour pair for a given log severity string.
///
/// Material 3's ColorScheme has no semantic slot for warn/info/debug, so
/// the swatches are kept here as design tokens. This file is inside
/// `lib/shared/theme/` so it is exempt from the no-raw-colours gate; do
/// not move these constants to a feature file.
///
/// `ERROR` is the only level that maps to a ColorScheme equivalent —
/// callers should prefer `Theme.of(context).colorScheme.errorContainer /
/// error` directly. The entry here exists for completeness so the helper
/// covers every value the API may return.
LogSeverityColors logSeverityColors(String severity) {
  switch (severity.toUpperCase()) {
    case 'ERROR':
      // Approximates Material 3 errorContainer / error. Prefer the
      // ColorScheme values at call sites when a BuildContext is in scope.
      return (bg: const Color(0xFFFFDAD6), fg: const Color(0xFFBA1A1A));
    case 'WARN':
      return (bg: Colors.orange.shade100, fg: Colors.orange.shade800);
    case 'INFO':
      return (bg: Colors.blue.shade100, fg: Colors.blue.shade800);
    case 'DEBUG':
      return (bg: Colors.grey.shade200, fg: Colors.grey.shade700);
    default:
      // TRACE and any other value.
      return (bg: Colors.purple.shade50, fg: Colors.purple.shade700);
  }
}
