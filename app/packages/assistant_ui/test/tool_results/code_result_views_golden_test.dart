@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';
import 'contract_fixtures_test.dart' show fixturesDir, fixturesOf;

/// `CodeFileResultView` or `CodeDiffResultView`, whichever [kind] names, built
/// from a shared fixture.
Widget _view(String kind, Map<String, Object?> data) {
  final Widget view = kind == 'code_file'
      ? CodeFileResultView(data: const CodeFileRenderer().parse(data)!)
      : CodeDiffResultView(data: const CodeDiffRenderer().parse(data)!);
  return SizedBox(
    width: 360,
    child: Padding(padding: const EdgeInsets.all(8), child: view),
  );
}

String _name(String kind, String file) {
  final stem = file.replaceFirst('valid-', '').replaceFirst('.json', '');
  return '${kind}_${stem.replaceAll('-', '_')}';
}

void main() {
  final dir = fixturesDir();
  final cases = <String, Map<String, Object?>>{
    for (final kind in ['code_file', 'code_diff'])
      for (final entry in fixturesOf(dir, kind).entries)
        if (entry.key.startsWith('valid-')) _name(kind, entry.key): entry.value,
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          _view(entry.value['kind']! as String, entry.value),
          size: const Size(380, 260),
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 250));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile(
            '../goldens/tool_result_${entry.key}_${brightness.name}.png',
          ),
        );
      });
    }
  }
}
