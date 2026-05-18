import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'chat_provider.dart';
import 'turn_status_label.dart';

/// Stall threshold — once the in-flight turn has been silent for this
/// long, the card transitions from its activity label to a stall label
/// with an elapsed-time indicator. Tuneable; documented in design.md
/// Decision 3 ("30 seconds is longer than the average human attention
/// span, shorter than most enterprise web tool calls").
const Duration kTurnStallThreshold = Duration(seconds: 30);

/// Feature flag for the stall-state "Skip" button that triggers
/// `POST /api/conversations/{c}/turns/{t}/cancel`.
///
/// Default `true` — the cancel endpoint is live (PR #853). Flip to
/// `false` as a kill switch if the cancel path misbehaves in the wild;
/// the visual stall card remains regardless, only the affordance to
/// cancel from the UI is gated. Per
/// `openspec/changes/turn-status-endpoint/design.md` open question 1
/// (gate "for one release while we measure cancel reliability").
const bool kSkipButtonEnabled = true;

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
            if (stalled && kSkipButtonEnabled)
              Padding(
                padding: const EdgeInsets.only(left: 8),
                child: TextButton(
                  key: const Key('turn_progress_card_skip_button'),
                  onPressed: () {
                    // Fire-and-forget: the SSE stream will surface the
                    // server-side cancellation as `agent_error`, which the
                    // chat notifier translates into normal post-turn
                    // cleanup. We deliberately do NOT await — the user
                    // wants the button to feel instantaneous.
                    unawaited(
                      ref.read(chatProvider.notifier).requestCancelTurn(),
                    );
                  },
                  style: TextButton.styleFrom(
                    foregroundColor: colorScheme.onErrorContainer,
                    padding: const EdgeInsets.symmetric(
                      horizontal: 12,
                      vertical: 4,
                    ),
                    minimumSize: Size.zero,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                  child: const Text('Skip', style: TextStyle(fontSize: 13)),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
