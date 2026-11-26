@Tags(['golden'])
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'golden.dart';

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('harness smoke golden (${brightness.name})', (tester) async {
      await pumpGolden(
        tester,
        const SizedBox(width: 40, height: 40),
        size: const Size(120, 80),
        brightness: brightness,
      );
      await tester.pump(const Duration(milliseconds: 50));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile('goldens/harness_smoke_${brightness.name}.png'),
      );
    });
  }
}
