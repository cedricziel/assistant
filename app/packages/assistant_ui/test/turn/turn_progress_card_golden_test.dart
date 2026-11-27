@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

TurnStatusSnapshot _snap(
  TurnEventKind kind, {
  String? tool,
  Duration age = const Duration(seconds: 2),
}) => TurnStatusSnapshot(
  runId: 'run-1',
  lastEventKind: kind,
  lastEventAt: kGoldenNow.subtract(age),
  currentToolName: tool,
);

void main() {
  final cases = <String, TurnStatusSnapshot>{
    'turn_progress_card_active': _snap(
      TurnEventKind.toolResult,
      tool: 'web-search',
    ),
    'turn_progress_card_stalled': _snap(
      TurnEventKind.thinking,
      age: const Duration(seconds: 95),
    ),
  };

  // Awaiting an approval: five minutes of silence, still no stall label.
  final awaiting = _snap(
    TurnEventKind.toolResult,
    age: const Duration(minutes: 5),
  );

  for (final entry in {
    ...cases,
    'turn_progress_card_awaiting': awaiting,
  }.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          TurnProgressCard(
            status: entry.value,
            awaitingLabel: entry.key == 'turn_progress_card_awaiting'
                ? 'Waiting for your approval'
                : null,
            onSkip: () {},
            now: () => kGoldenNow,
          ),
          size: const Size(420, 90),
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
