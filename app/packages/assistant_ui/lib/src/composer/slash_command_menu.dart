import 'package:material_ui/material_ui.dart';

import '../models/command_entry.dart';
import '../theme/assistant_ui_spacing.dart';

/// Floating menu of slash commands.
///
/// Shown above the input while the text starts with `/`. Lists the [commands]
/// whose name starts with [filter] and calls [onSelect] when one is tapped.
/// The menu never dispatches a command; the caller decides what selecting
/// one means.
class SlashCommandMenu extends StatelessWidget {
  const SlashCommandMenu({
    super.key,
    required this.commands,
    required this.filter,
    required this.onSelect,
  });

  /// All available commands.
  final List<CommandEntry> commands;

  /// Current text after the `/` (for example "mo" for "/mo").
  final String filter;

  /// Called when the user taps a command.
  final ValueChanged<CommandEntry> onSelect;

  @override
  Widget build(BuildContext context) {
    final filtered = commands.where((c) => c.name.startsWith(filter)).toList();

    if (filtered.isEmpty) return const SizedBox.shrink();

    final colorScheme = Theme.of(context).colorScheme;

    return Material(
      elevation: 4,
      borderRadius: BorderRadius.circular(8),
      color: colorScheme.surface,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 240),
        child: ListView.builder(
          shrinkWrap: true,
          padding: const EdgeInsets.symmetric(vertical: AssistantUiSpacing.xs),
          itemCount: filtered.length,
          itemBuilder: (context, index) {
            final cmd = filtered[index];
            return InkWell(
              onTap: () => onSelect(cmd),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: AssistantUiSpacing.md,
                  vertical: AssistantUiSpacing.sm,
                ),
                child: Row(
                  children: [
                    Text(
                      '/${cmd.name}',
                      style: TextStyle(
                        fontWeight: FontWeight.w600,
                        color: colorScheme.onSurface,
                      ),
                    ),
                    const SizedBox(width: AssistantUiSpacing.md),
                    Expanded(
                      child: Text(
                        cmd.description,
                        style: TextStyle(
                          color: colorScheme.onSurfaceVariant,
                          fontSize: 13,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}
