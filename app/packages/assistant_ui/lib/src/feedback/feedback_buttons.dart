import 'package:material_ui/material_ui.dart';

import '../message_actions.dart';
import '../models/message_rating.dart';

/// Thumbs up and thumbs down for rating an assistant reply.
///
/// The button matching [rating] is selected. Tapping a button reports its
/// rating through [onChanged]; tapping the selected one reports `null`, which
/// clears the rating. With [enabled] false the buttons stay visible but ignore
/// taps. The element holds no state and never calls anything itself.
///
/// Built from [MessageActionButton] so it sits in a [MessageActions] row. The
/// buttons carry the keys `feedback_positive_action` and
/// `feedback_negative_action`.
class FeedbackButtons extends StatelessWidget {
  const FeedbackButtons({
    super.key,
    required this.rating,
    required this.onChanged,
    this.enabled = true,
  });

  final MessageRating? rating;
  final ValueChanged<MessageRating?> onChanged;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 4,
      runSpacing: 4,
      children: [
        _button(
          key: const Key('feedback_positive_action'),
          target: MessageRating.positive,
          label: 'Good response',
          icon: Icons.thumb_up_outlined,
          selectedIcon: Icons.thumb_up,
        ),
        _button(
          key: const Key('feedback_negative_action'),
          target: MessageRating.negative,
          label: 'Bad response',
          icon: Icons.thumb_down_outlined,
          selectedIcon: Icons.thumb_down,
        ),
      ],
    );
  }

  Widget _button({
    required Key key,
    required MessageRating target,
    required String label,
    required IconData icon,
    required IconData selectedIcon,
  }) {
    final selected = rating == target;
    return MessageActionButton(
      key: key,
      icon: selected ? selectedIcon : icon,
      label: label,
      selected: selected,
      onPressed: enabled ? () => onChanged(selected ? null : target) : null,
    );
  }
}
