import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _renderer = DataTableResultRenderer();

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

Widget _view({
  List<Map<String, Object?>> rows = _rows,
  bool truncated = false,
  int? totalRows,
}) {
  final model = _renderer.parse({
    'kind': 'data_table',
    'columns': _columns,
    'rows': rows,
    'truncated': truncated,
    'total_rows': totalRows ?? rows.length,
  })!;
  return Padding(
    padding: const EdgeInsets.all(16),
    child: Builder(
      builder: (context) => _renderer.build(context, model, ToolCallStatus.ok),
    ),
  );
}

final dataTableResultViewComponent = WidgetbookComponent(
  name: 'DataTableResultView',
  useCases: [
    WidgetbookUseCase(name: 'tasks', builder: (context) => _view()),
    WidgetbookUseCase(
      name: 'truncated',
      builder: (context) => _view(truncated: true, totalRows: 500),
    ),
    WidgetbookUseCase(
      name: 'empty',
      builder: (context) => _view(rows: const []),
    ),
  ],
);
