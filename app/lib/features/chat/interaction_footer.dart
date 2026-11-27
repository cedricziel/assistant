import 'dart:async';

import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/models/interaction_answer.dart';
import 'interaction_kinds.dart';

/// The footer of a timeline entry that carries an interaction: the card of
/// its kind. While the interaction is open it rebuilds every second so the
/// countdown moves.
class InteractionFooter extends StatefulWidget {
  const InteractionFooter({
    super.key,
    required this.record,
    required this.onAnswer,
    this.clock = DateTime.now,
  });

  final InteractionRecord record;
  final ValueChanged<InteractionAnswer> onAnswer;
  final DateTime Function() clock;

  @override
  State<InteractionFooter> createState() => _InteractionFooterState();
}

class _InteractionFooterState extends State<InteractionFooter> {
  Timer? _ticker;

  @override
  void initState() {
    super.initState();
    _syncTicker();
  }

  @override
  void didUpdateWidget(InteractionFooter oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncTicker();
  }

  @override
  void dispose() {
    _ticker?.cancel();
    super.dispose();
  }

  void _syncTicker() {
    if (!widget.record.isOpen) {
      _ticker?.cancel();
      _ticker = null;
      return;
    }
    _ticker ??= Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) setState(() {});
    });
  }

  @override
  Widget build(BuildContext context) {
    final card = buildInteractionCard(
      widget.record,
      widget.clock(),
      widget.onAnswer,
    );
    if (card == null) return const SizedBox.shrink();
    return Padding(padding: const EdgeInsets.only(top: 8), child: card);
  }
}
