@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';
import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

Widget _view(Map<String, Object?> data) => SizedBox(
  width: 360,
  child: Padding(
    padding: const EdgeInsets.all(8),
    child: LinkPreviewResultView(
      data: const LinkPreviewRenderer().parse(data)!,
    ),
  ),
);

String _name(String file) => file
    .replaceFirst('valid-', '')
    .replaceFirst('.json', '')
    .replaceAll('-', '_');

void main() {
  final dir = fixturesDir();
  final cases = <String, Map<String, Object?>>{
    for (final entry in fixturesOf(dir, 'link_preview').entries)
      if (entry.key.startsWith('valid-')) _name(entry.key): entry.value,
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('LinkPreviewResultView ${entry.key} (${brightness.name})', (
        tester,
      ) async {
        await pumpGolden(
          tester,
          _view(entry.value),
          size: const Size(400, 240),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 250));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile(
            '../goldens/tool_result_link_preview_${entry.key}_${brightness.name}.png',
          ),
        );
      });
    }
  }
}
