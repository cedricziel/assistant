@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _reasoning =
    'The user wants a summary of the report. I should read the file first, '
    'then pull out the three main findings.';

Widget _body(Widget child) =>
    ThreadDensityScope(density: TimelineDensity.normal, child: child);

void main() {
  // Builders, because a token stream can be listened to only once.
  final cases = <String, ({Widget Function() widget, Size size, bool expand})>{
    // Active entries start expanded and show the streamed chunks.
    'thinking_body_active': (
      widget: () => _body(
        ThinkingBody(
          entryState: EntryState.active,
          tokenStream: Stream.value(_reasoning),
        ),
      ),
      size: const Size(420, 120),
      expand: false,
    ),
    // Complete entries collapse; tap to read the plain-text reasoning.
    'thinking_body_complete': (
      widget: () => _body(
        const ThinkingBody(
          entryState: EntryState.complete,
          content: _reasoning,
        ),
      ),
      size: const Size(420, 150),
      expand: true,
    ),
    'thinking_body_elapsed': (
      widget: () => _body(
        const ThinkingBody(
          entryState: EntryState.complete,
          content: _reasoning,
          elapsed: Duration(seconds: 4),
        ),
      ),
      size: const Size(420, 50),
      expand: false,
    ),
    'thinking_body_empty': (
      widget: () => _body(
        const ThinkingBody(entryState: EntryState.complete, content: ''),
      ),
      size: const Size(420, 50),
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
          // The first pump starts the expand animation, the second ends it.
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
