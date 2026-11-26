import 'dart:async';

import 'package:material_ui/material_ui.dart';

import '../models/turn_status.dart';
import '../theme/assistant_ui_spacing.dart';
import 'turn_status_label.dart';

/// Stall threshold: once the in-flight turn has been silent for this long, the
/// card swaps its activity label for a stall label with an elapsed-time
/// indicator.
const Duration kTurnStallThreshold = Duration(seconds: 30);

/// Compact, persistent indicator of the active turn's most recent activity.
///
/// Renders nothing when [status] is `null`. Otherwise shows the activity label
/// from [turnStatusLabel]; once [now] is [kTurnStallThreshold] past
/// [TurnStatusSnapshot.lastEventAt] it shows [stalledTurnStatusLabel] on an
/// error-tinted surface, and, when [showSkip] is set, a Skip button that calls
/// [onSkip].
///
/// [now] is the clock. The card reads it once a second while [status] is
/// non-null, so a stall appears without a new event; tests inject a fixed one.
class TurnProgressCard extends StatefulWidget {
  const TurnProgressCard({
    super.key,
    required this.status,
    required this.onSkip,
    this.showSkip = true,
    this.now = DateTime.now,
  });

  /// The active turn's latest activity, or `null` when no turn is in flight.
  final TurnStatusSnapshot? status;

  /// Whether the stalled state offers the Skip button.
  final bool showSkip;

  /// Called when the user activates Skip.
  final VoidCallback onSkip;

  /// The clock the stall check reads.
  final DateTime Function() now;

  @override
  State<TurnProgressCard> createState() => _TurnProgressCardState();
}

class _TurnProgressCardState extends State<TurnProgressCard> {
  Timer? _ticker;
  late DateTime _now = widget.now();

  @override
  void initState() {
    super.initState();
    _syncTicker();
  }

  @override
  void didUpdateWidget(TurnProgressCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    _now = widget.now();
    _syncTicker();
  }

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  void _syncTicker() {
    if (widget.status == null) {
      _ticker?.cancel();
      _ticker = null;
      return;
    }
    _ticker ??= Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() => _now = widget.now());
    });
  }

  @override
  Widget build(BuildContext context) {
    final status = widget.status;
    if (status == null) return const SizedBox.shrink();

    final elapsed = _now.difference(status.lastEventAt);
    final stalled = elapsed >= kTurnStallThreshold;
    final label = stalled
        ? stalledTurnStatusLabel(elapsed)
        : turnStatusLabel(status);

    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: AssistantUiSpacing.md,
        vertical: AssistantUiSpacing.xs,
      ),
      child: Container(
        padding: const EdgeInsets.symmetric(
          horizontal: AssistantUiSpacing.md,
          vertical: AssistantUiSpacing.sm,
        ),
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
            const SizedBox(width: AssistantUiSpacing.md),
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
            if (stalled && widget.showSkip)
              Padding(
                padding: const EdgeInsets.only(left: AssistantUiSpacing.sm),
                child: TextButton(
                  key: const Key('turn_progress_card_skip_button'),
                  onPressed: widget.onSkip,
                  style: TextButton.styleFrom(
                    foregroundColor: colorScheme.onErrorContainer,
                    padding: const EdgeInsets.symmetric(
                      horizontal: AssistantUiSpacing.md,
                      vertical: AssistantUiSpacing.xs,
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
