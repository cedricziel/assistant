import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import 'chat_provider.dart';
import 'turn_status_label.dart';

/// Stall threshold — once the in-flight turn has been silent for this
/// long, the card transitions from its activity label to a stall label
/// with an elapsed-time indicator. Tuneable; documented in design.md
/// Decision 3 ("30 seconds is longer than the average human attention
/// span, shorter than most enterprise web tool calls").
const Duration kTurnStallThreshold = Duration(seconds: 30);

/// Compact, persistent indicator of the active turn's most recent
/// activity. Renders nothing when no turn is in flight.
///
/// Driven by [ChatState.currentTurnStatus]. The label flips per SSE
/// event kind via [turnStatusLabel]; after [kTurnStallThreshold] of
/// silence, the card surfaces a stall message with elapsed-seconds via
/// a 1-second [Timer.periodic].
class TurnProgressCard extends ConsumerStatefulWidget {
  const TurnProgressCard({super.key});

  @override
  ConsumerState<TurnProgressCard> createState() => _TurnProgressCardState();
}

class _TurnProgressCardState extends ConsumerState<TurnProgressCard> {
  Timer? _ticker;
  DateTime _now = DateTime.now();

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  void _ensureTicker(TurnStatusSnapshot? snap) {
    if (snap == null) {
      _ticker?.cancel();
      _ticker = null;
      return;
    }
    _ticker ??= Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() => _now = DateTime.now());
    });
  }

  @override
  Widget build(BuildContext context) {
    final snap = ref.watch(
      chatProvider.select((s) => s.value?.currentTurnStatus),
    );
    _ensureTicker(snap);

    if (snap == null) {
      return const SizedBox.shrink();
    }

    final elapsed = _now.difference(snap.lastEventAt);
    final stalled = elapsed >= kTurnStallThreshold;
    final label = stalled
        ? stalledTurnStatusLabel(elapsed)
        : turnStatusLabel(snap);

    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: stalled
              ? colorScheme.errorContainer.withValues(alpha: 0.35)
              : colorScheme.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(
            color: stalled
                ? colorScheme.error.withValues(alpha: 0.4)
                : colorScheme.outlineVariant,
          ),
        ),
        child: Row(
          children: [
            SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator.adaptive(
                strokeWidth: 2,
                valueColor: AlwaysStoppedAnimation<Color>(
                  stalled ? colorScheme.error : colorScheme.primary,
                ),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                label,
                key: const Key('turn_progress_card_label'),
                style: TextStyle(
                  fontSize: 13,
                  color: stalled
                      ? colorScheme.onErrorContainer
                      : colorScheme.onSurface,
                ),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
