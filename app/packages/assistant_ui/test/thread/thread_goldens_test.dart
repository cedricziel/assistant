@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

void main() {
  final cases = <String, ({Widget widget, Size size})>{
    'thread_error_banner': (
      widget: ThreadErrorBanner(
        message: 'The connection to the server was lost.',
        onDismiss: () {},
      ),
      size: const Size(420, 60),
    ),
    'thread_empty_state': (
      widget: const ThreadEmptyState(),
      size: const Size(420, 300),
    ),
    'thread_empty_state_starters': (
      widget: ThreadEmptyState(
        starters: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: StarterPrompts(
            prompts: const ['Plan my week', 'Review the open pull requests'],
            onSelected: (_) {},
          ),
        ),
      ),
      size: const Size(420, 420),
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
        await tester.pump(const Duration(milliseconds: 100));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
