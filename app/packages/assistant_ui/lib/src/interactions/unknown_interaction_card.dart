import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// Notice for an interaction whose kind this build does not know.
///
/// The turn is still waiting on its owner, but nothing here can answer it, so
/// the card is inert: no buttons and no callbacks. It only tells the person
/// that a newer app is needed.
class UnknownInteractionCard extends StatelessWidget {
  const UnknownInteractionCard({super.key});

  static const _message = 'This request needs a newer version of the app.';

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;

    return Semantics(
      container: true,
      label: _message,
      excludeSemantics: true,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: scheme.surfaceContainerLow,
          border: Border.all(color: scheme.outlineVariant),
          borderRadius: BorderRadius.circular(AssistantUiSpacing.md),
        ),
        child: Padding(
          padding: const EdgeInsets.all(AssistantUiSpacing.md),
          child: Row(
            children: [
              Icon(
                Icons.system_update_alt,
                size: 18,
                color: scheme.onSurfaceVariant,
              ),
              const SizedBox(width: AssistantUiSpacing.sm),
              Expanded(
                child: Text(
                  _message,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: scheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
