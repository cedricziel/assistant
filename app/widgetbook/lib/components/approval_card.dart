import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

const _allOptions = [
  ApprovalDecision.allowOnce,
  ApprovalDecision.allowAlways,
  ApprovalDecision.rejectOnce,
  ApprovalDecision.rejectAlways,
];

InteractionRecord _record({
  InteractionStatus status = InteractionStatus.pending,
  ApprovalDecision? decision,
  List<ApprovalDecision> options = _allOptions,
}) => InteractionRecord(
  id: 'i1',
  kind: InteractionKind.toolApproval,
  status: status,
  createdAt: _now,
  expiresAt: _now.add(const Duration(minutes: 9, seconds: 42)),
  toolName: 'bash',
  arguments: const {'command': 'git status --short', 'cwd': '/repo'},
  reason: "no rule allows 'bash'",
  options: options,
  suggestedRule: 'bash(git status*)',
  decision: decision,
);

WidgetbookUseCase _useCase(String name, InteractionRecord record) =>
    WidgetbookUseCase(
      name: name,
      builder: (context) => Padding(
        padding: const EdgeInsets.all(16),
        child: ApprovalCard(now: _now, onDecision: (_) {}, record: record),
      ),
    );

final approvalCardComponent = WidgetbookComponent(
  name: 'ApprovalCard',
  useCases: [
    WidgetbookUseCase(
      name: 'default',
      builder: (context) {
        final status = enumKnob(
          context,
          label: 'status',
          values: InteractionStatus.values,
          initial: InteractionStatus.pending,
        );
        final decision = enumKnob(
          context,
          label: 'decision (resolved)',
          values: ApprovalDecision.values,
          initial: ApprovalDecision.allowOnce,
        );
        return Padding(
          padding: const EdgeInsets.all(16),
          child: ApprovalCard(
            now: _now,
            onDecision: (_) {},
            record: InteractionRecord(
              id: 'i1',
              kind: InteractionKind.toolApproval,
              status: status,
              createdAt: _now,
              expiresAt: _now.add(const Duration(minutes: 9, seconds: 42)),
              toolName: 'bash',
              arguments: const {
                'command': 'git status --short',
                'cwd': '/repo',
              },
              reason: "no rule allows 'bash'",
              options: _allOptions,
              suggestedRule: 'bash(git status*)',
              decision: status == InteractionStatus.resolved ? decision : null,
            ),
          ),
        );
      },
    ),
    _useCase('pending', _record()),
    _useCase(
      'pending, once only',
      _record(
        options: const [
          ApprovalDecision.allowOnce,
          ApprovalDecision.rejectOnce,
        ],
      ),
    ),
    _useCase('submitting', _record(status: InteractionStatus.submitting)),
    _useCase(
      'allowed once',
      _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.allowOnce,
      ),
    ),
    _useCase(
      'allowed always',
      _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.allowAlways,
      ),
    ),
    _useCase(
      'rejected once',
      _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.rejectOnce,
      ),
    ),
    _useCase(
      'rejected always',
      _record(
        status: InteractionStatus.resolved,
        decision: ApprovalDecision.rejectAlways,
      ),
    ),
    _useCase('expired', _record(status: InteractionStatus.expired)),
    _useCase('cancelled', _record(status: InteractionStatus.cancelled)),
  ],
);
