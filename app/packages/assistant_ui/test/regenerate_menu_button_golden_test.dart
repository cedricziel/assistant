@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

const _models = [
  RegenerateModel(name: 'model-a', isDefault: true),
  RegenerateModel(name: 'model-b'),
  RegenerateModel(name: 'model-c'),
];

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('RegenerateMenuButton closed ${brightness.name}', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        Padding(
          padding: const EdgeInsets.all(12),
          child: RegenerateMenuButton(models: _models, onSelected: (_) {}),
        ),
        size: const Size(300, 60),
        brightness: brightness,
      );

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/regenerate_menu_button_closed_${brightness.name}.png',
        ),
      );
    });

    testWidgets('RegenerateMenuButton open ${brightness.name}', (tester) async {
      await pumpGolden(
        tester,
        Padding(
          padding: const EdgeInsets.all(12),
          child: RegenerateMenuButton(models: _models, onSelected: (_) {}),
        ),
        size: const Size(300, 220),
        brightness: brightness,
      );
      await tester.tap(find.byType(PopupMenuButton<String>));
      await tester.pumpAndSettle();

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/regenerate_menu_button_open_${brightness.name}.png',
        ),
      );
    });
  }
}
