@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _columns = [
  {'key': 'id', 'label': 'ID'},
  {'key': 'name', 'label': 'Task'},
  {'key': 'next_run', 'label': 'Next run'},
  {'key': 'runs', 'label': 'Runs', 'type': 'number'},
];

const _rows = [
  {'id': 't1', 'name': 'Backup', 'next_run': '2026-01-16 02:00', 'runs': 12},
  {'id': 't2', 'name': 'Digest', 'next_run': '2026-01-16 08:30', 'runs': 140},
  {'id': 't3', 'name': 'Cleanup', 'next_run': null, 'runs': 3},
];

Map<String, Object?> _data({
  List<Map<String, Object?>> rows = _rows,
  bool truncated = false,
  int? totalRows,
}) => {
  'kind': 'data_table',
  'columns': _columns,
  'rows': rows,
  'truncated': truncated,
  'total_rows': totalRows ?? rows.length,
};

Widget _view(Map<String, Object?> data) {
  const renderer = DataTableResultRenderer();
  final model = renderer.parse(data)!;
  return SizedBox(
    width: 360,
    child: Builder(
      builder: (context) => renderer.build(context, model, ToolCallStatus.ok),
    ),
  );
}

void main() {
  final cases = <String, Map<String, Object?>>{
    'data_table_result_view_tasks': _data(),
    'data_table_result_view_truncated': _data(truncated: true, totalRows: 500),
    'data_table_result_view_empty': _data(rows: const []),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('DataTableResultView ${entry.key} (${brightness.name})', (
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
          matchesGoldenFile('../goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
