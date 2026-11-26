import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// A one-line [summary] that a tap swaps for the [expanded] full view.
///
/// Used at compact density, where a result view would crowd the thread. The
/// expansion is local state: it resets when the widget leaves the tree.
class CompactExpandable extends StatefulWidget {
  const CompactExpandable({
    super.key,
    required this.summary,
    required this.expanded,
    this.initiallyExpanded = false,
  });

  /// The collapsed one-line form.
  final Widget summary;

  /// The full view shown after the user expands.
  final Widget expanded;

  /// Whether the full view shows on first build. For the gallery and tests;
  /// the app leaves it collapsed.
  final bool initiallyExpanded;

  @override
  State<CompactExpandable> createState() => _CompactExpandableState();
}

class _CompactExpandableState extends State<CompactExpandable> {
  late bool _open = widget.initiallyExpanded;

  void _toggle() => setState(() => _open = !_open);

  @override
  Widget build(BuildContext context) {
    if (_open) return _expanded(context);
    final colors = Theme.of(context).colorScheme;
    return InkWell(
      excludeFromSemantics: true,
      onTap: _toggle,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: AssistantUiSpacing.xs),
        child: Row(
          children: [
            Expanded(child: widget.summary),
            const SizedBox(width: AssistantUiSpacing.sm),
            Semantics(
              container: true,
              button: true,
              label: 'Show full result',
              onTap: _toggle,
              excludeSemantics: true,
              child: Icon(
                Icons.expand_more,
                size: 18,
                color: colors.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _expanded(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      widget.expanded,
      Semantics(
        container: true,
        button: true,
        label: 'Hide full result',
        onTap: _toggle,
        excludeSemantics: true,
        child: TextButton.icon(
          onPressed: _toggle,
          icon: const Icon(Icons.expand_less, size: 18),
          label: const Text('Hide full result'),
        ),
      ),
    ],
  );
}
