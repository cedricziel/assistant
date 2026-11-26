import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _edit = <String, Object?>{
  'kind': 'code_diff',
  'path': 'src/lib.rs',
  'language': 'rust',
  'operation': 'edit',
  'hunks': [
    {
      'old_start': 3,
      'old_lines': 3,
      'new_start': 3,
      'new_lines': 3,
      'lines': [
        {'op': 'context', 'text': 'fn a() {'},
        {'op': 'remove', 'text': '    1'},
        {'op': 'add', 'text': '    2'},
        {'op': 'context', 'text': '}'},
      ],
    },
  ],
  'added': 1,
  'removed': 1,
  'truncated': false,
};

const _create = <String, Object?>{
  'kind': 'code_diff',
  'path': 'new.txt',
  'operation': 'create',
  'hunks': [
    {
      'old_start': 0,
      'old_lines': 0,
      'new_start': 1,
      'new_lines': 2,
      'lines': [
        {'op': 'add', 'text': 'one'},
        {'op': 'add', 'text': 'two'},
      ],
    },
  ],
  'added': 2,
  'removed': 0,
  'truncated': false,
};

/// Fails the contract: `delete` is not an operation.
const _invalid = <String, Object?>{
  'kind': 'code_diff',
  'path': 'a',
  'operation': 'delete',
  'hunks': <Object?>[],
  'added': 0,
  'removed': 0,
  'truncated': false,
};

Widget _result(
  Map<String, Object?> data, {
  ToolCallStatus status = ToolCallStatus.ok,
  TimelineDensity density = TimelineDensity.normal,
}) => Padding(
  padding: const EdgeInsets.all(16),
  child: ThreadDensityScope(
    density: density,
    child: Builder(
      builder: (context) {
        final record = ToolCallRecord(
          toolName: 'file-edit',
          status: status,
          data: data,
        );
        return ToolResultRendererRegistry(byKind: defaultToolResultRenderers())
                .build(context, record) ??
            GenericToolResultView(record: record);
      },
    ),
  ),
);

final codeDiffResultViewComponent = WidgetbookComponent(
  name: 'CodeDiffResultView',
  useCases: [
    WidgetbookUseCase(name: 'edit', builder: (context) => _result(_edit)),
    WidgetbookUseCase(name: 'create', builder: (context) => _result(_create)),
    WidgetbookUseCase(
      name: 'truncated',
      builder: (context) => _result({..._edit, 'truncated': true}),
    ),
    WidgetbookUseCase(
      name: 'invalid falls back',
      builder: (context) => _result(_invalid),
    ),
    WidgetbookUseCase(
      name: 'error status',
      builder: (context) => _result(_edit, status: ToolCallStatus.error),
    ),
    for (final density in TimelineDensity.values)
      WidgetbookUseCase(
        name: 'density ${density.name}',
        builder: (context) => _result(_edit, density: density),
      ),
  ],
);
