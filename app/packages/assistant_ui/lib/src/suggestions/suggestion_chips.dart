import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// Follow-up prompts offered after a turn, one tappable chip each.
///
/// Chip `i` carries the key `suggestion_chip_<i>`. A tap calls [onSelected]
/// with the prompt; the element never sends anything itself. With
/// [enabled] false the chips stay visible but ignore taps, for example while
/// a turn is running. An empty [prompts] list renders nothing.
class SuggestionChips extends StatelessWidget {
  const SuggestionChips({
    super.key,
    required this.prompts,
    required this.onSelected,
    this.enabled = true,
  });

  final List<String> prompts;
  final ValueChanged<String> onSelected;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    if (prompts.isEmpty) return const SizedBox.shrink();

    return Semantics(
      container: true,
      label: 'Suggested follow-ups',
      child: LayoutBuilder(
        builder: (context, constraints) => Wrap(
          spacing: AssistantUiSpacing.sm,
          runSpacing: AssistantUiSpacing.xs,
          children: [
            for (var i = 0; i < prompts.length; i++)
              ConstrainedBox(
                constraints: BoxConstraints(maxWidth: constraints.maxWidth),
                child: ActionChip(
                  key: Key('suggestion_chip_$i'),
                  label: Text(
                    prompts[i],
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                  onPressed: enabled ? () => onSelected(prompts[i]) : null,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
