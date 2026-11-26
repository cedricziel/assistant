@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

Widget _expandable({required bool initiallyExpanded}) => SizedBox(
  width: 340,
  child: CompactExpandable(
    initiallyExpanded: initiallyExpanded,
    summary: const Text('exit 2 · 14 lines stderr'),
    expanded: const Text('full result view\nsecond line of the full view'),
  ),
);

void main() {
  final cases = <String, bool>{
    'compact_expandable_collapsed': false,
    'compact_expandable_expanded': true,
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          _expandable(initiallyExpanded: entry.value),
          size: const Size(380, 120),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 250));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('../goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
