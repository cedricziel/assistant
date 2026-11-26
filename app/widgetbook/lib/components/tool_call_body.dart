import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

ToolCallRecord _record(ToolCallStatus status) => ToolCallRecord(
  toolName: 'web-search',
  status: status,
  arguments: const {'query': 'flutter golden tests', 'limit': 3},
  result: switch (status) {
    ToolCallStatus.pending => null,
    ToolCallStatus.ok => '3 results found',
    ToolCallStatus.error => 'Request failed: connection refused',
    ToolCallStatus.denied => 'Denied by permission policy',
    ToolCallStatus.cancelled => 'Cancelled by user',
  },
  duration: status == ToolCallStatus.pending
      ? null
      : const Duration(milliseconds: 1200),
);

EntryState _entryState(ToolCallStatus status) =>
    status == ToolCallStatus.pending ? EntryState.active : EntryState.complete;

Widget _body(ToolCallStatus status, {Widget? footer}) => ToolCallBody(
  record: _record(status),
  entryState: _entryState(status),
  footer: footer,
);

final toolCallBodyComponent = WidgetbookComponent(
  name: 'ToolCallBody',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => ThreadDensityScope(
        density: enumKnob(
          context,
          label: 'density',
          values: TimelineDensity.values,
          initial: TimelineDensity.normal,
        ),
        child: _body(
          enumKnob(
            context,
            label: 'status',
            values: ToolCallStatus.values,
            initial: ToolCallStatus.ok,
          ),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'status matrix',
      builder: (context) => SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [for (final status in ToolCallStatus.values) _body(status)],
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'with footer',
      builder: (context) => _body(
        ToolCallStatus.ok,
        footer: const Card(
          child: Padding(
            padding: EdgeInsets.all(12),
            child: Text('An app frame or approval card renders here.'),
          ),
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'custom resolver',
      builder: (context) => ToolResultRendererScope(
        resolve: (context, record) => Text('3 results for ${record.toolName}'),
        child: _body(ToolCallStatus.error),
      ),
    ),
  ],
);
