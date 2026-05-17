import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import 'chat_provider.dart';

/// Visible representation of a [PendingMessage] sitting in
/// [ChatState.pendingQueue].
///
/// Rendered as a ghosted user bubble — muted text colour, no avatar,
/// a "Queued" badge — so the user can see exactly what's queued and
/// in what order. Long-press (mobile) / right-click (desktop) opens
/// an [AdaptiveActionSheet] with a "Remove from queue" action.
///
/// Specified by `openspec/changes/chat-stream-progress-ux/specs/.../
/// chat-message-queue/spec.md` — "Queued messages render as visible
/// ghost bubbles" and "Queued messages are removable before they send".
class QueuedMessageBubble extends ConsumerWidget {
  const QueuedMessageBubble({
    super.key,
    required this.message,
    required this.index,
  });

  /// The queued message to render.
  final PendingMessage message;

  /// Position of [message] in `pendingQueue`. Used by the remove action.
  final int index;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colorScheme = Theme.of(context).colorScheme;
    final mutedFg = colorScheme.onSurfaceVariant;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onLongPress: () => _showRemoveSheet(context, ref),
        onSecondaryTap: () => _showRemoveSheet(context, ref),
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

  Future<void> _showRemoveSheet(BuildContext context, WidgetRef ref) async {
    await showAdaptiveActionSheet<void>(
      context: context,
      title: 'Queued message',
      message: message.text,
      actions: [
        AdaptiveActionSheetAction(
          label: 'Remove from queue',
          isDestructive: true,
          onPressed: (ctx) {
            ref.read(chatProvider.notifier).removeFromQueue(index);
            Navigator.of(ctx).pop();
          },
        ),
      ],
    );
  }
}
