@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

InteractionRecord _record({
  InteractionStatus status = InteractionStatus.pending,
  ApprovalDecision? decision,
}) => InteractionRecord(
  id: 'i1',
  kind: InteractionKind.toolApproval,
  status: status,
  createdAt: kGoldenNow,
  expiresAt: kGoldenNow.add(const Duration(minutes: 9, seconds: 42)),
  toolName: 'bash',
  arguments: const {'command': 'git status --short'},
  reason: "no rule allows 'bash'",
  options: ApprovalDecision.values,
  suggestedRule: 'bash(git status*)',
  decision: decision,
);

void main() {
  final cases = <String, ({InteractionRecord record, Size size})>{
    'approval_card_pending': (record: _record(), size: const Size(400, 520)),
    'approval_card_allowed_always': (
      record: _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.allowAlways,
      ),
      size: const Size(400, 120),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          Padding(
            padding: const EdgeInsets.all(16),
            child: ApprovalCard(
              record: entry.value.record,
              now: kGoldenNow,
              onDecision: (_) {},
            ),
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
