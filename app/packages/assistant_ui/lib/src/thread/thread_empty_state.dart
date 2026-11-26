import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// Centered placeholder for a conversation with no messages.
///
/// [starters] is a slot below the subtitle for per-Persona starter prompts
/// (`StarterPrompts`); it renders nothing when `null`.
class ThreadEmptyState extends StatelessWidget {
  const ThreadEmptyState({
    super.key,
    this.title = 'Start a conversation',
    this.subtitle = 'Type a message below to begin.',
    this.starters,
  });

  final String title;
  final String subtitle;
  final Widget? starters;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.chat_bubble_outline,
            size: 64,
            color: colorScheme.outlineVariant,
          ),
          const SizedBox(height: AssistantUiSpacing.lg),
          Text(
            title,
            style: TextStyle(
              fontSize: 18,
              color: colorScheme.onSurfaceVariant,
              fontWeight: FontWeight.w500,
            ),
          ),
          const SizedBox(height: AssistantUiSpacing.sm),
          Text(subtitle, style: TextStyle(color: colorScheme.onSurfaceVariant)),
          if (starters != null) ...[
            const SizedBox(height: AssistantUiSpacing.xl),
            starters!,
          ],
        ],
      ),
    );
  }
}
