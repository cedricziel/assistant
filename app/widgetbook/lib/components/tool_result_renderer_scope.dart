import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

ToolCallRecord get _record => ToolCallRecord(
  toolName: 'web-search',
  // An error status expands the details, so the resolved view is visible.
  status: ToolCallStatus.error,
  arguments: const {'query': 'flutter'},
  result: 'Request failed: connection refused',
);

Widget _entry() => ToolCallBody(record: _record);

final toolResultRendererScopeComponent = WidgetbookComponent(
  name: 'ToolResultRendererScope',
  useCases: [
    WidgetbookUseCase(
      name: 'resolver replaces the generic view',
      builder: (context) => ToolResultRendererScope(
        resolve: (context, record) =>
            Text('Custom view for ${record.toolName}'),
        child: _entry(),
      ),
    ),
    WidgetbookUseCase(
      name: 'resolver returns null (falls back)',
      builder: (context) => ToolResultRendererScope(
        resolve: (context, record) => null,
        child: _entry(),
      ),
    ),
    WidgetbookUseCase(
      name: 'no scope (generic view)',
      builder: (context) => _entry(),
    ),
  ],
);
