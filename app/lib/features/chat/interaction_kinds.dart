import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';

import '../../api/models/interaction_answer.dart';

/// Builds the card of one interaction kind. [now] drives the countdown and
/// [onAnswer] sends the owner's answer to the turn.
typedef InteractionCardBuilder = Widget Function(
  InteractionRecord record,
  DateTime now,
  ValueChanged<InteractionAnswer> onAnswer,
);

/// How the chat shows one interaction kind: what the progress card says while
/// the turn waits, what is announced, and the card that renders in the tool
/// call's footer.
class InteractionKindSpec {
  const InteractionKindSpec({
    required this.progressLabel,
    required this.announcement,
    this.card,
  });

  /// Shown by the progress card in place of the activity label.
  final String progressLabel;

  /// Spoken once when the interaction is requested.
  final String announcement;

  /// The footer card, or null for a kind that renders elsewhere.
  final InteractionCardBuilder? card;
}

/// Kinds the app knows. Others fall back to [_unregistered].
final Map<InteractionKind, InteractionKindSpec> _kinds = {
  InteractionKind.toolApproval: InteractionKindSpec(
    progressLabel: 'Waiting for your approval',
    announcement: 'Approval needed',
    card: (record, now, onAnswer) => ApprovalCard(
      record: record,
      now: now,
      onDecision: (decision) => onAnswer(ToolApprovalAnswer(decision)),
    ),
  ),
};

const _unregistered = InteractionKindSpec(
  progressLabel: 'Waiting for your input',
  announcement: 'Input needed',
);

/// The spec for [record]'s kind, or the generic one when it is not registered.
InteractionKindSpec interactionKindSpec(InteractionRecord record) =>
    _kinds[record.kind] ?? _unregistered;

/// The footer card for [record], or null when it renders no card.
///
/// A kind that wants no card gets none. A card kind this build cannot answer
/// gets the inert [UnknownInteractionCard], never an automatic answer.
Widget? buildInteractionCard(
  InteractionRecord record,
  DateTime now,
  ValueChanged<InteractionAnswer> onAnswer,
) {
  if (!record.rendersCard) return null;
  final spec = _kinds[record.kind];
  if (spec == null) return const UnknownInteractionCard();
  return spec.card?.call(record, now, onAnswer);
}
