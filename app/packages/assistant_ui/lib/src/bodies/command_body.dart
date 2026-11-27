import 'package:material_ui/material_ui.dart';

/// A system-event style row for a slash-command acknowledgement: a terminal
/// icon, the command name (for example `/model`) and the acknowledgement text
/// in one compact centered row. It is not a chat bubble and has no shell.
class CommandBody extends StatelessWidget {
  const CommandBody({super.key, required this.name, required this.ack});

  /// Key of the row, for tests that identify it.
  static const tileKey = Key('command_event_tile');

  /// The command name without its leading slash.
  final String name;

  /// The acknowledgement the command produced.
  final String ack;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Padding(
      key: tileKey,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.terminal, size: 16, color: colorScheme.onSurfaceVariant),
          const SizedBox(width: 6),
          Text(
            '/$name',
            style: TextStyle(
              fontWeight: FontWeight.w600,
              fontSize: 13,
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              ack,
              style: TextStyle(
                fontSize: 13,
                color: colorScheme.onSurfaceVariant,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
