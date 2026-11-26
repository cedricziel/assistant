@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

Widget _padded(Widget child) =>
    Padding(padding: const EdgeInsets.all(16), child: child);

void main() {
  FeedbackButtons buttons(MessageRating? rating, {bool enabled = true}) =>
      FeedbackButtons(rating: rating, onChanged: (_) {}, enabled: enabled);

  FeedbackCommentField field({String? initialText, bool sending = false}) =>
      FeedbackCommentField(
        initialText: initialText,
        sending: sending,
        onSubmit: (_) {},
        onDismiss: () {},
      );

  final cases = <String, ({Widget widget, Size size})>{
    'feedback_buttons_none': (widget: buttons(null), size: const Size(420, 64)),
    'feedback_buttons_positive': (
      widget: buttons(MessageRating.positive),
      size: const Size(420, 64),
    ),
    'feedback_buttons_negative': (
      widget: buttons(MessageRating.negative),
      size: const Size(420, 64),
    ),
    'feedback_buttons_disabled': (
      widget: buttons(MessageRating.positive, enabled: false),
      size: const Size(420, 64),
    ),
    'feedback_comment_field_empty': (
      widget: field(),
      size: const Size(420, 80),
    ),
    'feedback_comment_field_prefilled': (
      widget: field(initialText: 'Wrong file, it should be the config.'),
      size: const Size(420, 80),
    ),
    'feedback_comment_field_sending': (
      widget: field(initialText: 'Clear and short.', sending: true),
      size: const Size(420, 80),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          _padded(entry.value.widget),
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
