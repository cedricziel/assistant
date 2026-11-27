import 'package:material_ui/material_ui.dart';

import 'theme/assistant_ui_spacing.dart';

/// Version switcher for a message that has sibling versions: "‹ 2/3 ›".
///
/// [index] is zero-based. The previous control is disabled at the first
/// version and the next control at the last. Both are disabled while [busy]
/// (a switch is in flight). Screen readers hear "Version 2 of 3".
class BranchPicker extends StatelessWidget {
  const BranchPicker({
    super.key,
    required this.index,
    required this.count,
    required this.onPrevious,
    required this.onNext,
    this.busy = false,
    this.model,
  }) : assert(count > 0, 'count must be positive'),
       assert(index >= 0 && index < count, 'index must be within count');

  final int index;
  final int count;
  final VoidCallback onPrevious;
  final VoidCallback onNext;
  final bool busy;

  /// The model that produced the active version, when known and different
  /// from the default. Shown as the picker's tooltip, so the model behind a
  /// regenerated version is discoverable.
  final String? model;

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(context).textTheme.labelMedium;
    final hasPrevious = !busy && index > 0;
    final hasNext = !busy && index < count - 1;

    final row = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        IconButton(
          tooltip: 'Previous version',
          icon: const Icon(Icons.chevron_left),
          iconSize: AssistantUiSpacing.lg + AssistantUiSpacing.xs,
          visualDensity: VisualDensity.compact,
          onPressed: hasPrevious ? onPrevious : null,
        ),
        ExcludeSemantics(child: Text('${index + 1}/$count', style: style)),
        IconButton(
          tooltip: 'Next version',
          icon: const Icon(Icons.chevron_right),
          iconSize: AssistantUiSpacing.lg + AssistantUiSpacing.xs,
          visualDensity: VisualDensity.compact,
          onPressed: hasNext ? onNext : null,
        ),
      ],
    );

    return Semantics(
      container: true,
      label: 'Version ${index + 1} of $count',
      child: model == null ? row : Tooltip(message: model, child: row),
    );
  }
}
