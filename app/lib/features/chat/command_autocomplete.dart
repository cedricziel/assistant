import 'package:flutter/material.dart';

import 'commands_provider.dart';

/// Floating autocomplete popup for slash commands.
///
/// Shown above the input field when the user types `/` as the first character.
/// Filters commands by prefix and invokes [onSelect] when a command is tapped.
class CommandAutocompletePopup extends StatelessWidget {
  const CommandAutocompletePopup({
    super.key,
    required this.commands,
    required this.filter,
    required this.onSelect,
  });

  /// All available commands.
  final List<CommandEntry> commands;

  /// Current text after the `/` (e.g. "mo" for "/mo").
  final String filter;

  /// Called when the user taps a command.
  final void Function(CommandEntry command) onSelect;

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
          padding: const EdgeInsets.symmetric(vertical: 4),
          itemCount: filtered.length,
          itemBuilder: (context, index) {
            final cmd = filtered[index];
            return InkWell(
              onTap: () => onSelect(cmd),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
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
                    const SizedBox(width: 12),
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
