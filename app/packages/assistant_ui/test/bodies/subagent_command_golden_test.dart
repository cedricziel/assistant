@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

ToolCallRecord _call(String name, ToolCallStatus status) =>
    ToolCallRecord(toolName: name, status: status);

Widget _body(Widget child, [TimelineDensity d = TimelineDensity.expanded]) =>
    ThreadDensityScope(density: d, child: child);

void main() {
  final cases = <String, ({Widget Function() widget, Size size, bool expand})>{
    'subagent_body_running': (
      widget: () => _body(
        SubagentBody(
          label: 'Research authentication',
          entryState: EntryState.active,
          thinking: 'Analyzing OAuth providers...',
          toolCalls: [
            _call('web-search', ToolCallStatus.ok),
            _call('file-read', ToolCallStatus.error),
            _call('shell', ToolCallStatus.pending),
          ],
        ),
      ),
      size: const Size(420, 160),
      expand: false,
    ),
    'subagent_body_running_normal': (
      widget: () => _body(
        SubagentBody(
          label: 'Research authentication',
          entryState: EntryState.active,
          toolCalls: [
            _call('web-search', ToolCallStatus.ok),
            _call('file-read', ToolCallStatus.ok),
            _call('shell', ToolCallStatus.pending),
          ],
        ),
        TimelineDensity.normal,
      ),
      size: const Size(420, 120),
      expand: false,
    ),
    'subagent_body_complete': (
      widget: () => _body(
        const SubagentBody(
          label: 'Research caching',
          summary: 'Found three prior approaches.',
        ),
      ),
      size: const Size(420, 90),
      expand: true,
    ),
    'subagent_body_stale': (
      widget: () => _body(
        const SubagentBody(
          label: 'Research caching',
          summary: 'Found three prior approaches.',
          entryState: EntryState.stale,
        ),
      ),
      size: const Size(420, 50),
      expand: false,
    ),
    'command_body': (
      widget: () =>
          const CommandBody(name: 'model', ack: 'Model set to gpt-4o'),
      size: const Size(420, 40),
      expand: false,
    ),
    'command_body_long_ack': (
      widget: () => CommandBody(
        name: 'compact',
        ack: 'History compacted: ${'older turns summarised ' * 6}',
      ),
      size: const Size(320, 40),
      expand: false,
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          entry.value.widget(),
          size: entry.value.size,
          brightness: brightness,
        );
        if (entry.value.expand) {
          await tester.tap(find.byKey(TimelineEntryShell.headerKey));
          await tester.pump(const Duration(milliseconds: 300));
        }
        await tester.pump(const Duration(milliseconds: 300));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
