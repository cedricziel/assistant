import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

ToolCallRecord _call(String name, ToolCallStatus status) =>
    ToolCallRecord(toolName: name, status: status);

final subagentBodyComponent = WidgetbookComponent(
  name: 'SubagentBody',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => ThreadDensityScope(
        density: enumKnob(
          context,
          label: 'density',
          values: TimelineDensity.values,
          initial: TimelineDensity.expanded,
        ),
        child: SubagentBody(
          label: context.knobs.string(
            label: 'task',
            initialValue: 'Research authentication',
          ),
          entryState: enumKnob(
            context,
            label: 'entry state',
            values: EntryState.values,
            initial: EntryState.active,
          ),
          summary: context.knobs.boolean(label: 'complete')
              ? 'Found three prior approaches.'
              : null,
          thinking: 'Analyzing OAuth providers...',
          toolCalls: [
            _call('web-search', ToolCallStatus.ok),
            _call('file-read', ToolCallStatus.error),
            _call('shell', ToolCallStatus.pending),
          ],
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'complete',
      builder: (context) => const SubagentBody(
        label: 'Research caching',
        summary: 'Found three prior approaches.',
      ),
    ),
    WidgetbookUseCase(
      name: 'stale',
      builder: (context) => const SubagentBody(
        label: 'Research caching',
        summary: 'Found three prior approaches.',
        entryState: EntryState.stale,
      ),
    ),
  ],
);
