@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

Widget _shell({
  required EntryState state,
  TimelineDensity density = TimelineDensity.normal,
  bool showBody = false,
  Widget? footer,
}) => ThreadDensityScope(
  density: density,
  child: TimelineEntryShell(
    icon: Icons.build_outlined,
    label: 'web-search',
    status: state == EntryState.active
        ? const SizedBox(width: 12, height: 12)
        : const Icon(Icons.check_circle_outline, size: 14),
    entryState: state,
    expandable: true,
    forceExpand: showBody,
    body: const Text('query: flutter golden tests'),
    footer: footer,
  ),
);

Widget get _approvalFooter => Padding(
  padding: const EdgeInsets.only(top: 4),
  child: Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      FilledButton(onPressed: () {}, child: const Text('Allow')),
      const SizedBox(width: 8),
      OutlinedButton(onPressed: () {}, child: const Text('Deny')),
    ],
  ),
);

void main() {
  final cases = <String, ({Widget widget, Size size})>{
    'timeline_entry_shell_active': (
      widget: _shell(state: EntryState.active),
      size: const Size(420, 110),
    ),
    'timeline_entry_shell_complete': (
      widget: _shell(state: EntryState.complete, showBody: true),
      size: const Size(420, 110),
    ),
    'timeline_entry_shell_stale': (
      widget: _shell(state: EntryState.stale, showBody: true),
      size: const Size(420, 110),
    ),
    'timeline_entry_shell_awaiting_input': (
      widget: _shell(state: EntryState.awaitingInput, footer: _approvalFooter),
      size: const Size(420, 150),
    ),
    'timeline_entry_shell_compact_active': (
      widget: _shell(
        state: EntryState.active,
        density: TimelineDensity.compact,
      ),
      size: const Size(420, 70),
    ),
    'timeline_entry_shell_collapsed_footer': (
      widget: _shell(state: EntryState.complete, footer: _approvalFooter),
      size: const Size(420, 110),
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

  for (final brightness in Brightness.values) {
    testWidgets('timeline_entry_shell_header_focused (${brightness.name})', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        _shell(state: EntryState.complete),
        size: const Size(420, 110),
        brightness: brightness,
      );
      await tester.sendKeyEvent(LogicalKeyboardKey.tab);
      await tester.pump(const Duration(milliseconds: 250));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/timeline_entry_shell_header_focused_${brightness.name}.png',
        ),
      );
    });
  }
}
