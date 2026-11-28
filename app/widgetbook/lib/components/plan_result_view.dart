import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

const _plan = <String, Object?>{
  'kind': 'plan',
  'title': 'Ship the feature',
  'steps': [
    {'title': 'Write tests', 'status': 'completed'},
    {'title': 'Implement', 'status': 'in_progress'},
    {'title': 'Review', 'status': 'pending'},
    {'title': 'Deploy', 'status': 'failed'},
    {'title': 'Announce', 'status': 'skipped'},
  ],
};

const _empty = <String, Object?>{'kind': 'plan', 'steps': <Object?>[]};

/// Fails the contract: `done` is not a status.
const _invalid = <String, Object?>{
  'kind': 'plan',
  'steps': [
    {'title': 'Write tests', 'status': 'done'},
  ],
};

Widget _result(
  Map<String, Object?> data, {
  ToolCallStatus status = ToolCallStatus.ok,
  TimelineDensity density = TimelineDensity.normal,
}) => Padding(
  padding: const EdgeInsets.all(16),
  child: ThreadDensityScope(
    density: density,
    child: Builder(
      builder: (context) {
        final record = ToolCallRecord(
          toolName: 'plan-update',
          status: status,
          data: data,
        );
        return ToolResultRendererRegistry(byKind: defaultToolResultRenderers())
                .build(context, record) ??
            GenericToolResultView(record: record);
      },
    ),
  ),
);

final planResultViewComponent = WidgetbookComponent(
  name: 'PlanResultView',
  useCases: [
    WidgetbookUseCase(name: 'plan', builder: (context) => _result(_plan)),
    WidgetbookUseCase(name: 'empty', builder: (context) => _result(_empty)),
    WidgetbookUseCase(
      name: 'invalid falls back',
      builder: (context) => _result(_invalid),
    ),
    WidgetbookUseCase(
      name: 'error status',
      builder: (context) => _result(_plan, status: ToolCallStatus.error),
    ),
    for (final density in TimelineDensity.values)
      WidgetbookUseCase(
        name: 'density ${density.name}',
        builder: (context) => _result(_plan, density: density),
      ),
  ],
);
