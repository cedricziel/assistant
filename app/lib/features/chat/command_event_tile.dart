import 'package:flutter/material.dart';

import 'chat_provider.dart';

/// A system-event style tile for slash-command events in the chat timeline.
///
/// Shows a terminal icon, the command name (e.g. `/model`), and the
/// acknowledgement text in a compact centered row.
class CommandEventTile extends StatelessWidget {
  const CommandEventTile({super.key, required this.message});

  final ChatMessage message;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final name = message.commandName ?? '';
    final ack = message.commandAckText ?? message.content;

    return Padding(
      key: const Key('command_event_tile'),
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
