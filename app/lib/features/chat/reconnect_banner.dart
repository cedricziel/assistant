import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import 'chat_provider.dart';

/// Slim, transient banner shown while [ChatNotifier.attemptReconnect] is
/// recovering an interrupted stream (typically after
/// `AppLifecycleState.resumed`).
///
/// Renders nothing when `isReconnecting` is false — i.e. on routine
/// app-resume transitions where there's no interrupted stream to
/// recover. Only the recovery path that does network work (replay +
/// loadConversation) flips the flag, so the banner doesn't flash on
/// every foreground.
class ReconnectBanner extends ConsumerWidget {
  const ReconnectBanner({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isReconnecting = ref.watch(
      chatProvider.select((s) => s.value?.isReconnecting ?? false),
    );

    if (!isReconnecting) return const SizedBox.shrink();

    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.secondaryContainer,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          key: const Key('reconnect_banner'),
          children: [
            SizedBox(
              width: 14,
              height: 14,
              child: CircularProgressIndicator.adaptive(
                strokeWidth: 2,
                valueColor: AlwaysStoppedAnimation<Color>(
                  colorScheme.onSecondaryContainer,
                ),
              ),
            ),
            const SizedBox(width: 10),
            Text(
              'Reconnecting…',
              style: TextStyle(
                fontSize: 13,
                color: colorScheme.onSecondaryContainer,
                fontWeight: FontWeight.w500,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
