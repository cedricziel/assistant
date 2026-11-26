import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _valid = <String, Object?>{
  'kind': 'code_file',
  'path': 'src/main.rs',
  'language': 'rust',
  'content': 'fn main() {\n    println!("hello");\n}\n',
  'truncated': false,
  'total_chars': 40,
};

const _truncated = <String, Object?>{
  'kind': 'code_file',
  'path': 'notes.txt',
  'content': 'first part of a long file',
  'truncated': true,
  'total_chars': 90000,
};

/// Fails the contract: `total_chars` must be an integer.
const _invalid = <String, Object?>{
  'kind': 'code_file',
  'path': 'a',
  'content': '',
  'truncated': false,
  'total_chars': '0',
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
        final built =
            ToolResultRendererRegistry(
              byKind: defaultToolResultRenderers(),
            ).build(
              context,
              ToolCallRecord(toolName: 'file-read', status: status, data: data),
            );
        return built ??
            GenericToolResultView(
              record: ToolCallRecord(
                toolName: 'file-read',
                status: status,
                data: data,
              ),
            );
      },
    ),
  ),
);

final codeFileResultViewComponent = WidgetbookComponent(
  name: 'CodeFileResultView',
  useCases: [
    WidgetbookUseCase(name: 'valid', builder: (context) => _result(_valid)),
    WidgetbookUseCase(
      name: 'truncated',
      builder: (context) => _result(_truncated),
    ),
    WidgetbookUseCase(
      name: 'invalid falls back',
      builder: (context) => _result(_invalid),
    ),
    WidgetbookUseCase(
      name: 'error status',
      builder: (context) => _result(_valid, status: ToolCallStatus.error),
    ),
    for (final density in TimelineDensity.values)
      WidgetbookUseCase(
        name: 'density ${density.name}',
        builder: (context) => _result(_valid, density: density),
      ),
  ],
);
