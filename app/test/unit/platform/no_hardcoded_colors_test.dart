import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Lint-style test that ensures no hard-coded light-only colors remain in
/// the app source code. These colors break in dark mode and signal
/// non-native platform feel.
void main() {
  test('no hard-coded Colors.black* in app/lib/', () {
    final violations = _findViolations(
      RegExp(r'Colors\.black\d*'),
      'Colors.black',
    );
    expect(
      violations,
      isEmpty,
      reason:
          'Found hard-coded Colors.black* — use colorScheme tokens instead:\n'
          '${violations.join('\n')}',
    );
  });

  test('no hard-coded Colors.red* in app/lib/', () {
    final violations = _findViolations(RegExp(r'Colors\.red'), 'Colors.red');
    expect(
      violations,
      isEmpty,
      reason:
          'Found hard-coded Colors.red* — use colorScheme.error tokens instead:\n'
          '${violations.join('\n')}',
    );
  });

  test('no hard-coded Colors.white in app/lib/', () {
    final violations = _findViolations(
      RegExp(r'Colors\.white\b'),
      'Colors.white',
    );
    expect(
      violations,
      isEmpty,
      reason:
          'Found hard-coded Colors.white — use colorScheme tokens instead:\n'
          '${violations.join('\n')}',
    );
  });
}

/// Walks `app/lib/` and returns a list of "$file:$line: $match" strings
/// for every line matching [pattern].
List<String> _findViolations(RegExp pattern, String label) {
  final libDir = Directory('lib');
  if (!libDir.existsSync()) return ['lib/ directory not found'];

  final violations = <String>[];
  for (final entity in libDir.listSync(recursive: true)) {
    if (entity is! File || !entity.path.endsWith('.dart')) continue;
    final lines = entity.readAsLinesSync();
    for (var i = 0; i < lines.length; i++) {
      final line = lines[i];
      // Skip comments.
      if (line.trimLeft().startsWith('//')) continue;
      if (pattern.hasMatch(line)) {
        violations.add('${entity.path}:${i + 1}: ${line.trim()}');
      }
    }
  }
  return violations;
}
