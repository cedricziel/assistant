import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

const _models = [
  RegenerateModel(name: 'model-a', isDefault: true),
  RegenerateModel(name: 'model-b'),
];

void main() {
  group('RegenerateMenuButton', () {
    testWidgets('renders nothing with fewer than two models', (tester) async {
      for (final models in [
        const <RegenerateModel>[],
        _models.take(1).toList(),
      ]) {
        await pumpGolden(
          tester,
          RegenerateMenuButton(models: models, onSelected: (_) {}),
        );

        expect(find.byType(PopupMenuButton<String>), findsNothing);
        expect(find.text('Regenerate with…'), findsNothing);
      }
    });

    testWidgets('selecting an entry fires onSelected with its name', (
      tester,
    ) async {
      final selected = <String>[];
      await pumpGolden(
        tester,
        Align(
          alignment: Alignment.topLeft,
          child: RegenerateMenuButton(
            models: _models,
            onSelected: selected.add,
          ),
        ),
      );

      await tester.tap(find.byType(PopupMenuButton<String>));
      await tester.pumpAndSettle();
      await tester.tap(find.text('model-b'));
      await tester.pumpAndSettle();

      expect(selected, ['model-b']);
    });

    testWidgets('lists every model and marks the default', (tester) async {
      await pumpGolden(
        tester,
        RegenerateMenuButton(models: _models, onSelected: (_) {}),
      );

      await tester.tap(find.byType(PopupMenuButton<String>));
      await tester.pumpAndSettle();

      expect(find.text('model-a'), findsOneWidget);
      expect(find.text('model-b'), findsOneWidget);
      expect(find.text('default'), findsOneWidget);
      expect(
        find.descendant(
          of: find.widgetWithText(PopupMenuItem<String>, 'model-a'),
          matching: find.text('default'),
        ),
        findsOneWidget,
        reason: 'the default marker belongs to the default model',
      );
    });

    testWidgets('exposes a button label to screen readers', (tester) async {
      final handle = tester.ensureSemantics();
      await pumpGolden(
        tester,
        RegenerateMenuButton(models: _models, onSelected: (_) {}),
      );

      expect(find.bySemanticsLabel('Regenerate with…'), findsOneWidget);
      handle.dispose();
    });
  });
}
