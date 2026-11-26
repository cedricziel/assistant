import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter/widgets.dart';
import 'package:widgetbook/widgetbook.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

Widget _card(TurnStatusSnapshot status) => Padding(
  padding: const EdgeInsets.all(16),
  child: TurnProgressCard(status: status, onSkip: () {}, now: () => _now),
);

final turnProgressCardComponent = WidgetbookComponent(
  name: 'TurnProgressCard',
  useCases: [
    WidgetbookUseCase(
      name: 'active',
      builder: (context) => _card(
        TurnStatusSnapshot(
          runId: 'run-1',
          lastEventKind: TurnEventKind.toolResult,
          lastEventAt: _now.subtract(const Duration(seconds: 2)),
          currentToolName: 'web-search',
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'stalled with skip',
      builder: (context) => _card(
        TurnStatusSnapshot(
          runId: 'run-1',
          lastEventKind: TurnEventKind.thinking,
          lastEventAt: _now.subtract(const Duration(seconds: 95)),
        ),
      ),
    ),
  ],
);
