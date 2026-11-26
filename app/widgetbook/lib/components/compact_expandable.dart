import 'package:assistant_ui/assistant_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

Widget _expandable({bool initiallyExpanded = false}) => Padding(
  padding: const EdgeInsets.all(16),
  child: CompactExpandable(
    // Remount when the knob flips so the new initial state applies.
    key: ValueKey(initiallyExpanded),
    initiallyExpanded: initiallyExpanded,
    summary: const Text('exit 2 · 14 lines stderr'),
    expanded: const Text('full result view\nsecond line of the full view'),
  ),
);

final compactExpandableComponent = WidgetbookComponent(
  name: 'CompactExpandable',
  useCases: [
    WidgetbookUseCase(
      name: 'playground',
      builder: (context) => _expandable(
        initiallyExpanded: context.knobs.boolean(label: 'initiallyExpanded'),
      ),
    ),
    WidgetbookUseCase(name: 'collapsed', builder: (context) => _expandable()),
    WidgetbookUseCase(
      name: 'expanded',
      builder: (context) => _expandable(initiallyExpanded: true),
    ),
  ],
);
