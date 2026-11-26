@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

void main() {
  const long =
      'Walk me through every step of the migration, including the rollback '
      'plan and what to monitor afterwards, then summarise the risks';

  final cases = <String, ({String text, Size size})>{
    'queued_message_bubble_short': (
      text: 'Summarise this thread',
      size: const Size(400, 80),
    ),
    'queued_message_bubble_long': (text: long, size: const Size(400, 120)),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          QueuedMessageBubble(
            message: PendingMessage(
              text: entry.value.text,
              conversationId: 'c',
            ),
            onLongPress: () {},
          ),
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
