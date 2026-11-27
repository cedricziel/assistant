import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _valid = <String, Object?>{
  'kind': 'terminal',
  'command': 'cargo test',
  'exit_code': 0,
  'stdout': 'running 12 tests\nall tests passed\n',
  'stderr': '',
  'stdout_truncated': false,
  'stderr_truncated': false,
  'duration_ms': 4210,
  'timed_out': false,
};

const _failure = <String, Object?>{
  'kind': 'terminal',
  'command': 'cargo test',
  'exit_code': 101,
  'stdout': 'running 12 tests\n',
  'stderr': 'error: test failed, to rerun pass `--lib`\n',
  'stdout_truncated': false,
  'stderr_truncated': false,
  'duration_ms': 4210,
  'timed_out': false,
};

/// Fails the contract: `exit_code` must be an integer or null.
const _invalid = <String, Object?>{
  'kind': 'terminal',
  'command': 'cargo test',
  'exit_code': 'zero',
  'stdout': 'running 12 tests\nall tests passed\n',
  'stderr': '',
  'stdout_truncated': false,
  'stderr_truncated': false,
  'duration_ms': 4210,
  'timed_out': false,
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
          toolName: 'bash',
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

final terminalResultViewComponent = WidgetbookComponent(
  name: 'TerminalResultView',
  useCases: [
    WidgetbookUseCase(name: 'valid', builder: (context) => _result(_valid)),
    WidgetbookUseCase(
      name: 'failure',
      builder: (context) => _result(_failure, status: ToolCallStatus.error),
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
        builder: (context) => _result(_failure, density: density),
      ),
  ],
);
