import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Width compared against a literal or a private `k…` constant.
final _widthComparison = RegExp(
  r'\b(width|maxWidth)\s*[<>]=?\s*([0-9]|_?k[A-Z]\w*)',
);

/// A private `…Breakpoint` constant.
final _breakpointConstant = RegExp(r'const double _?k\w*Breakpoint');

/// Files allowed to declare or compare layout widths. Scoped per file so a
/// new comparison elsewhere in `features/workflows` still fails the lint.
const _allowlist = {
  'shared/layout/breakpoints.dart',
  'features/workflows/workflow_editor_screen.dart',
};

bool _matches(String line) =>
    _widthComparison.hasMatch(line) || _breakpointConstant.hasMatch(line);

void main() {
  test('no literal layout widths remain under app/lib', () {
    final libDir = Directory('lib');
    final offenders = <String>[];
    for (final entity in libDir.listSync(recursive: true)) {
      if (entity is! File || !entity.path.endsWith('.dart')) continue;
      final relative = entity.path.substring('lib/'.length);
      if (_allowlist.contains(relative)) continue;
      final lines = entity.readAsLinesSync();
      for (var i = 0; i < lines.length; i++) {
        if (_matches(lines[i])) {
          offenders.add('$relative:${i + 1}: ${lines[i].trim()}');
        }
      }
    }
    expect(
      offenders,
      isEmpty,
      reason: 'Use Breakpoints.wide from shared/layout/breakpoints.dart',
    );
  });

  test('the allowlist is exactly the token file and the workflow editor', () {
    expect(_allowlist, {
      'shared/layout/breakpoints.dart',
      'features/workflows/workflow_editor_screen.dart',
    });
    for (final path in _allowlist) {
      expect(File('lib/$path').existsSync(), isTrue, reason: '$path exists');
    }
  });

  test('the matcher catches the forms in use today', () {
    for (final line in const [
      'size.width > 700',
      'MediaQuery.sizeOf(context).width >= _kNavRailBreakpoint',
      'constraints.maxWidth < 600',
      'const double _kNavRailBreakpoint = 768;',
    ]) {
      expect(_matches(line), isTrue, reason: line);
    }
  });
}
