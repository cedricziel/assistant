import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

Widget _view(ToolCallRecord record) => Padding(
  padding: const EdgeInsets.all(16),
  child: GenericToolResultView(record: record),
);

final genericToolResultViewComponent = WidgetbookComponent(
  name: 'GenericToolResultView',
  useCases: [
    WidgetbookUseCase(
      name: 'arguments and result',
      builder: (context) => _view(
        ToolCallRecord(
          toolName: 'bash',
          status: ToolCallStatus.ok,
          arguments: const {'command': 'ls -la'},
          result: 'total 8\ndrwxr-xr-x  2 user  staff  64 Jan 15 12:00 .',
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'error result',
      builder: (context) => _view(
        ToolCallRecord(
          toolName: 'bash',
          status: ToolCallStatus.error,
          arguments: const {'command': 'rm -rf /'},
          result: 'Permission denied: cannot delete root',
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'arguments only',
      builder: (context) => _view(
        ToolCallRecord(
          toolName: 'web-fetch',
          status: ToolCallStatus.pending,
          arguments: const {'url': 'https://example.com'},
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'with structured data',
      builder: (context) => _view(
        ToolCallRecord(
          toolName: 'file-glob',
          status: ToolCallStatus.ok,
          arguments: const {'pattern': 'lib/**/*.dart'},
          result: '3 files',
          data: const {
            'paths': ['lib/main.dart', 'lib/app.dart', 'test/app_test.dart'],
            'truncated': false,
          },
        ),
      ),
    ),
  ],
);
