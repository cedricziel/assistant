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
  List<ApprovalDecision> options = ApprovalDecision.values,
}) => InteractionRecord(
  id: 'i1',
  kind: InteractionKind.toolApproval,
  status: status,
  createdAt: kGoldenNow,
  expiresAt: kGoldenNow.add(const Duration(minutes: 9, seconds: 42)),
  toolName: 'bash',
  arguments: const {'command': 'git status --short'},
  reason: "no rule allows 'bash'",
  options: options,
  suggestedRule: 'bash(git status*)',
  decision: decision,
);

void main() {
  const pendingSize = Size(400, 520);
  const receiptSize = Size(400, 120);
  ({InteractionRecord record, Size size}) receipt(
    InteractionStatus status, [
    ApprovalDecision? decision,
  ]) =>
      (record: _record(status: status, decision: decision), size: receiptSize);

  final cases = <String, ({InteractionRecord record, Size size})>{
    'approval_card_pending': (record: _record(), size: pendingSize),
    'approval_card_pending_once_only': (
      record: _record(
        options: const [
          ApprovalDecision.allowOnce,
          ApprovalDecision.rejectOnce,
        ],
      ),
      size: const Size(400, 300),
    ),
    'approval_card_submitting': (
      record: _record(status: InteractionStatus.submitting),
      size: pendingSize,
    ),
    'approval_card_allowed_once': receipt(
      InteractionStatus.resolved,
      ApprovalDecision.allowOnce,
    ),
    'approval_card_allowed_always': receipt(
      InteractionStatus.resolved,
      ApprovalDecision.allowAlways,
    ),
    'approval_card_rejected_once': receipt(
      InteractionStatus.resolved,
      ApprovalDecision.rejectOnce,
    ),
    'approval_card_rejected_always': receipt(
      InteractionStatus.resolved,
      ApprovalDecision.rejectAlways,
    ),
    'approval_card_expired': receipt(InteractionStatus.expired),
    'approval_card_cancelled': receipt(InteractionStatus.cancelled),
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

  for (final brightness in Brightness.values) {
    testWidgets('unknown_interaction_card (${brightness.name})', (
      tester,
    ) async {
      await pumpGolden(
        tester,
        const Padding(
          padding: EdgeInsets.all(16),
          child: UnknownInteractionCard(),
        ),
        size: const Size(400, 120),
        brightness: brightness,
      );

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/unknown_interaction_card_${brightness.name}.png',
        ),
      );
    });
  }
}
