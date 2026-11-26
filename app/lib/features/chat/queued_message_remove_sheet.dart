import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import 'chat_provider.dart';

/// Opens the "Remove from queue" action sheet for the queued [message] at
/// [index] in `pendingQueue`.
///
/// Passed to `QueuedMessageBubble.onLongPress`: the element stays pure, and
/// the platform sheet and the [chatProvider] call stay in the app.
Future<void> showQueuedMessageRemoveSheet(
  BuildContext context,
  WidgetRef ref,
  PendingMessage message,
  int index,
) {
  return showAdaptiveActionSheet<void>(
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
