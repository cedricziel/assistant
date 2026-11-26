@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

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
  startedAt: kGoldenNow,
);

Widget _body(ToolCallStatus status, {Widget? footer}) => ThreadDensityScope(
  density: TimelineDensity.normal,
  child: ToolCallBody(
    record: _record(status),
    entryState: status == ToolCallStatus.pending
        ? EntryState.active
        : EntryState.complete,
    footer: footer,
  ),
);

Widget get _footer => const Padding(
  padding: EdgeInsets.only(top: 4),
  child: Card(
    child: Padding(padding: EdgeInsets.all(12), child: Text('App frame')),
  ),
);

void main() {
  final cases = <String, ({Widget widget, Size size})>{
    // Ok and denied collapse; pending is active and error force-expands.
    'tool_call_body_pending': (
      widget: _body(ToolCallStatus.pending),
      size: const Size(420, 140),
    ),
    'tool_call_body_ok': (
      widget: _body(ToolCallStatus.ok),
      size: const Size(420, 70),
    ),
    'tool_call_body_error': (
      widget: _body(ToolCallStatus.error),
      size: const Size(420, 200),
    ),
    'tool_call_body_denied': (
      widget: _body(ToolCallStatus.denied),
      size: const Size(420, 70),
    ),
    'tool_call_body_footer': (
      widget: _body(ToolCallStatus.ok, footer: _footer),
      size: const Size(420, 150),
    ),
    // ToolResultRendererScope with a custom resolver.
    'tool_call_body_custom_resolver': (
      widget: ToolResultRendererScope(
        resolve: (context, record) =>
            Text('Custom view for ${record.toolName}'),
        child: _body(ToolCallStatus.error),
      ),
      size: const Size(420, 110),
    ),
    'tool_status_icon_all': (
      widget: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          spacing: 16,
          children: [
            for (final status in ToolCallStatus.values)
              ToolStatusIcon(status: status),
          ],
        ),
      ),
      size: const Size(200, 50),
    ),
    'generic_tool_result_view': (
      widget: Padding(
        padding: const EdgeInsets.all(16),
        child: GenericToolResultView(record: _record(ToolCallStatus.ok)),
      ),
      size: const Size(420, 200),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          entry.value.widget,
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 250));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
