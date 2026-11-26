import 'dart:io';

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_widgetbook/components.dart';
import 'package:flutter_test/flutter_test.dart';

const _packageTests = '../packages/assistant_ui/test';
final _goldenTag = RegExp(r'''@Tags\(\s*\[[^\]]*['"]golden['"]''');

/// Registry entries that no gallery component is named after.
List<String> missingUseCases(
  Iterable<String> registry,
  Iterable<String> components,
) => registry.where((e) => !components.contains(e)).toList();

/// Registry entries that no `golden`-tagged test source mentions.
List<String> missingGoldens(
  Iterable<String> registry,
  Iterable<String> goldenSources,
) => registry
    .where((e) => !goldenSources.any((s) => RegExp('\\b$e\\b').hasMatch(s)))
    .toList();

void main() {
  group('missingUseCases', () {
    test('names entries without a component', () {
      expect(missingUseCases(['A', 'B'], ['A']), ['B']);
    });
  });

  group('missingGoldens', () {
    test('names entries no golden source mentions', () {
      expect(missingGoldens(['A', 'AB'], ['final x = A();']), ['AB']);
    });
  });

  group('kAssistantUiElements', () {
    test('each entry has a gallery use-case', () {
      final components = kComponents.map((c) => c.name);

      expect(
        missingUseCases(kAssistantUiElements, components),
        isEmpty,
        reason: 'registry entries without a widgetbook component',
      );
    });

    test('each entry has a golden-tagged test in the package', () {
      final sources = Directory(_packageTests)
          .listSync(recursive: true)
          .whereType<File>()
          .where((f) => f.path.endsWith('_test.dart'))
          .map((f) => f.readAsStringSync())
          .where(_goldenTag.hasMatch)
          .toList();

      expect(
        missingGoldens(kAssistantUiElements, sources),
        isEmpty,
        reason: 'registry entries without a golden-tagged test',
      );
    });
  });
}
