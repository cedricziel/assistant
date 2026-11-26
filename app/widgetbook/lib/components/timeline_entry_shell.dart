import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

import '../knobs.dart';

Widget _shell({
  required EntryState state,
  required TimelineDensity density,
  Widget? footer,
}) => ThreadDensityScope(
  density: density,
  child: TimelineEntryShell(
    key: ValueKey('${state.name}-${density.name}'),
    icon: Icons.build_outlined,
    label: 'web-search',
    status: const Icon(Icons.check_circle_outline, size: 14),
    entryState: state,
    expandable: true,
    body: const Text('query: flutter golden tests'),
    footer: footer,
  ),
);

Widget _approval() => Row(
  mainAxisSize: MainAxisSize.min,
  children: [
    FilledButton(onPressed: () {}, child: const Text('Allow')),
    const SizedBox(width: 8),
    OutlinedButton(onPressed: () {}, child: const Text('Deny')),
  ],
);

final timelineEntryShellComponent = WidgetbookComponent(
  name: 'TimelineEntryShell',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => _shell(
        state: enumKnob(
          context,
          label: 'entryState',
          values: EntryState.values,
          initial: EntryState.active,
        ),
        density: enumKnob(
          context,
          label: 'density',
          values: TimelineDensity.values,
          initial: TimelineDensity.normal,
        ),
        footer: context.knobs.boolean(label: 'footer') ? _approval() : null,
      ),
    ),
    WidgetbookUseCase(
      name: 'state × density matrix',
      builder: (context) => SingleChildScrollView(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            for (final state in EntryState.values)
              for (final density in TimelineDensity.values)
                _shell(state: state, density: density),
          ],
        ),
      ),
    ),
    WidgetbookUseCase(
      name: 'collapsed with footer',
      builder: (context) => _shell(
        state: EntryState.complete,
        density: TimelineDensity.normal,
        footer: _approval(),
      ),
    ),
    WidgetbookUseCase(
      name: 'awaiting input with footer',
      builder: (context) => _shell(
        state: EntryState.awaitingInput,
        density: TimelineDensity.normal,
        footer: _approval(),
      ),
    ),
  ],
);
