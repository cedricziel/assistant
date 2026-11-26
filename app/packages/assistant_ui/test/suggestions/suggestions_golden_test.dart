@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

Widget _padded(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

void main() {
  const starters = [
    'Plan my week around the meetings on my calendar',
    'Review the open pull requests',
    'Summarise the news from this morning',
  ];
  const longPrompt =
      'Walk me through every step of the migration, including the rollback '
      'plan and what to monitor afterwards';

  final cases = <String, ({Widget widget, Size size})>{
    'suggestion_chips_one': (
      widget: SuggestionChips(
        prompts: const ['Summarise this'],
        onSelected: (_) {},
      ),
      size: const Size(400, 96),
    ),
    'suggestion_chips_three': (
      widget: SuggestionChips(
        prompts: const [
          'Summarise this',
          'Draft a reply',
          'Explain in more detail',
        ],
        onSelected: (_) {},
      ),
      size: const Size(400, 120),
    ),
    'suggestion_chips_long': (
      widget: SuggestionChips(
        prompts: const [longPrompt, 'Thanks'],
        onSelected: (_) {},
      ),
      size: const Size(400, 160),
    ),
    'suggestion_chips_disabled': (
      widget: SuggestionChips(
        prompts: const ['Summarise this', 'Draft a reply'],
        onSelected: (_) {},
        enabled: false,
      ),
      size: const Size(400, 96),
    ),
    'starter_prompts_narrow': (
      widget: StarterPrompts(prompts: starters, onSelected: (_) {}),
      size: const Size(400, 300),
    ),
    'starter_prompts_wide': (
      widget: StarterPrompts(prompts: starters, onSelected: (_) {}),
      size: const Size(720, 220),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          _padded(entry.value.widget),
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
