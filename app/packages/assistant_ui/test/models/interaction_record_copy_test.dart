import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

InteractionRecord _record() => InteractionRecord(
  id: 'i-1',
  kind: InteractionKind.toolApproval,
  status: InteractionStatus.pending,
  createdAt: DateTime.utc(2026),
  expiresAt: DateTime.utc(2026, 1, 1, 0, 10),
  toolName: 'bash',
  options: const [ApprovalDecision.allowOnce, ApprovalDecision.rejectOnce],
);

void main() {
  group('InteractionRecord.copyWith', () {
    test('changes only the named fields', () {
      final resolved = _record().copyWith(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.allowAlways,
        ruleId: 'rule-1',
        resolvedAt: DateTime.utc(2026, 1, 1, 0, 1),
      );

      expect(resolved.status, InteractionStatus.resolved);
      expect(resolved.decision, ApprovalDecision.allowAlways);
      expect(resolved.ruleId, 'rule-1');
      expect(resolved.id, 'i-1');
      expect(resolved.toolName, 'bash');
      expect(resolved.options, hasLength(2));
    });
  });

  group('TurnStatusSnapshot.awaitingLabel', () {
    TurnStatusSnapshot snap() => TurnStatusSnapshot(
      runId: 'r',
      lastEventKind: TurnEventKind.status,
      lastEventAt: DateTime.utc(2026),
    );

    test('is kept by copyWith and cleared on request', () {
      final waiting = snap().copyWith(awaitingLabel: 'Waiting for your input');

      expect(
        waiting.copyWith(lastEventKind: TurnEventKind.token).awaitingLabel,
        'Waiting for your input',
      );
      expect(waiting.copyWith(clearAwaitingLabel: true).awaitingLabel, isNull);
    });
  });
}
