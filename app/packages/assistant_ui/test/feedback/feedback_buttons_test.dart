import 'dart:ui' show Tristate;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _positive = Key('feedback_positive_action');
const _negative = Key('feedback_negative_action');

Widget _host(
  MessageRating? rating,
  List<MessageRating?> changes, {
  bool enabled = true,
}) => MaterialApp(
  home: Scaffold(
    body: FeedbackButtons(
      rating: rating,
      onChanged: changes.add,
      enabled: enabled,
    ),
  ),
);

void main() {
  testWidgets('with no rating, tapping positive emits positive', (
    tester,
  ) async {
    final changes = <MessageRating?>[];
    await tester.pumpWidget(_host(null, changes));

    await tester.tap(find.byKey(_positive));

    expect(changes, [MessageRating.positive]);
  });

  testWidgets('tapping the selected negative rating emits null', (
    tester,
  ) async {
    final changes = <MessageRating?>[];
    await tester.pumpWidget(_host(MessageRating.negative, changes));

    await tester.tap(find.byKey(_negative));

    expect(changes, [null]);
  });

  testWidgets('tapping negative while positive emits negative', (tester) async {
    final changes = <MessageRating?>[];
    await tester.pumpWidget(_host(MessageRating.positive, changes));

    await tester.tap(find.byKey(_negative));

    expect(changes, [MessageRating.negative]);
  });

  testWidgets('only the matching button is selected in semantics', (
    tester,
  ) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(_host(MessageRating.positive, []));

    final good = tester.getSemantics(find.byKey(_positive));
    final bad = tester.getSemantics(find.byKey(_negative));

    expect(good.label, 'Good response');
    expect(good.flagsCollection.isSelected, Tristate.isTrue);
    expect(bad.label, 'Bad response');
    expect(bad.flagsCollection.isSelected, isNot(Tristate.isTrue));
    handle.dispose();
  });

  testWidgets('the selected rating shows a filled icon', (tester) async {
    await tester.pumpWidget(_host(MessageRating.positive, []));

    expect(find.byIcon(Icons.thumb_up), findsOneWidget);
    expect(find.byIcon(Icons.thumb_down_outlined), findsOneWidget);
  });

  testWidgets('enabled: false ignores taps', (tester) async {
    final changes = <MessageRating?>[];
    await tester.pumpWidget(_host(null, changes, enabled: false));

    await tester.tap(find.byKey(_positive));
    await tester.tap(find.byKey(_negative));

    expect(changes, isEmpty);
  });
}
