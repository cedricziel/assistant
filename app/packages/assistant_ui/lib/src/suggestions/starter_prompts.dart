import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// Per-Persona starter prompts for an empty conversation, laid out as a grid
/// of cards: one column below [twoColumnBreakpoint] dp, two from there up.
///
/// Card `i` carries the key `starter_prompt_<i>`. A tap calls [onSelected]
/// with the prompt. An empty [prompts] list renders nothing.
class StarterPrompts extends StatelessWidget {
  const StarterPrompts({
    super.key,
    required this.prompts,
    required this.onSelected,
  });

  /// Width, in dp, from which the cards use two columns.
  static const double twoColumnBreakpoint = 600;

  final List<String> prompts;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    if (prompts.isEmpty) return const SizedBox.shrink();

    return LayoutBuilder(
      builder: (context, constraints) {
        final columns = constraints.maxWidth >= twoColumnBreakpoint ? 2 : 1;
        return Semantics(
          container: true,
          label: 'Starter prompts',
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var start = 0; start < prompts.length; start += columns)
                Padding(
                  padding: EdgeInsets.only(
                    top: start == 0 ? 0 : AssistantUiSpacing.sm,
                  ),
                  child: IntrinsicHeight(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        for (var c = 0; c < columns; c++) ...[
                          if (c > 0)
                            const SizedBox(width: AssistantUiSpacing.sm),
                          Expanded(
                            child: start + c < prompts.length
                                ? _StarterCard(
                                    key: Key('starter_prompt_${start + c}'),
                                    prompt: prompts[start + c],
                                    onTap: () => onSelected(prompts[start + c]),
                                  )
                                : const SizedBox.shrink(),
                          ),
                        ],
                      ],
                    ),
                  ),
                ),
            ],
          ),
        );
      },
    );
  }
}

class _StarterCard extends StatelessWidget {
  const _StarterCard({super.key, required this.prompt, required this.onTap});

  final String prompt;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Card.outlined(
      margin: EdgeInsets.zero,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(AssistantUiSpacing.lg),
          child: Text(prompt, style: Theme.of(context).textTheme.bodyMedium),
        ),
      ),
    );
  }
}
