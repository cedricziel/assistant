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
  ],
);
