import 'package:material_ui/material_ui.dart';

import '../models/chat_message.dart';

/// Visible representation of a [PendingMessage] sitting in the send queue.
///
/// Rendered as a ghosted user bubble — muted text colour, no avatar, a
/// "Queued" badge — so the user can see exactly what is queued and in what
/// order. Long-press (mobile) and right-click (desktop) call [onLongPress];
/// the element opens nothing itself, so the app decides what removing a
/// queued message looks like.
///
/// Specified by `openspec/changes/chat-stream-progress-ux/specs/.../
/// chat-message-queue/spec.md` — "Queued messages render as visible ghost
/// bubbles" and "Queued messages are removable before they send".
class QueuedMessageBubble extends StatelessWidget {
  const QueuedMessageBubble({
    super.key,
    required this.message,
    required this.onLongPress,
  });

  /// The queued message to render.
  final PendingMessage message;

  /// Called on long-press and on secondary tap.
  final VoidCallback onLongPress;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final mutedFg = colorScheme.onSurfaceVariant;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onLongPress: onLongPress,
        onSecondaryTap: onLongPress,
        child: Align(
          alignment: AlignmentDirectional.centerEnd,
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 480),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.6,
                ),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: colorScheme.outlineVariant,
                  style: BorderStyle.solid,
                ),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Flexible(
                    child: Text(
                      message.text,
                      key: const Key('queued_message_bubble_text'),
                      style: TextStyle(
                        color: mutedFg,
                        fontStyle: FontStyle.italic,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 6,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Text(
                      'Queued',
                      style: TextStyle(
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                        color: mutedFg,
                        letterSpacing: 0.4,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
