import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _send = Key('feedback_comment_send');
const _input = Key('feedback_comment_input');

class _Harness {
  final submitted = <String>[];
  var dismissed = 0;

  Widget build({
    String? initialText,
    int maxLength = 2000,
    bool sending = false,
  }) => MaterialApp(
    home: Scaffold(
      body: FeedbackCommentField(
        initialText: initialText,
        onSubmit: submitted.add,
        onDismiss: () => dismissed++,
        maxLength: maxLength,
        sending: sending,
      ),
    ),
  );
}

bool _sendEnabled(WidgetTester tester) =>
    tester.widget<IconButton>(find.byKey(_send)).onPressed != null;

void main() {
  testWidgets('Send trims the text and fires onSubmit', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build());

    await tester.enterText(find.byKey(_input), ' missed the deadline ');
    await tester.pump();
    await tester.tap(find.byKey(_send));

    expect(h.submitted, ['missed the deadline']);
  });

  testWidgets('Send is disabled while the field is empty or blank', (
    tester,
  ) async {
    final h = _Harness();
    await tester.pumpWidget(h.build());
    expect(_sendEnabled(tester), isFalse);

    await tester.enterText(find.byKey(_input), '   ');
    await tester.pump();
    expect(_sendEnabled(tester), isFalse);

    await tester.enterText(find.byKey(_input), 'ok');
    await tester.pump();
    expect(_sendEnabled(tester), isTrue);
  });

  testWidgets('Send is disabled while sending', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build(initialText: 'text', sending: true));

    expect(_sendEnabled(tester), isFalse);
    await tester.tap(find.byKey(_send), warnIfMissed: false);
    expect(h.submitted, isEmpty);
  });

  testWidgets('Enter submits', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build());

    await tester.enterText(find.byKey(_input), 'clear and short');
    await tester.testTextInput.receiveAction(TextInputAction.send);
    await tester.pump();

    expect(h.submitted, ['clear and short']);
  });

  testWidgets('Enter on an empty field does not submit', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build());

    await tester.tap(find.byKey(_input));
    await tester.testTextInput.receiveAction(TextInputAction.send);
    await tester.pump();

    expect(h.submitted, isEmpty);
  });

  testWidgets('Escape dismisses without submitting', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build(initialText: 'draft'));

    await tester.tap(find.byKey(_input));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pump();

    expect(h.dismissed, 1);
    expect(h.submitted, isEmpty);
  });

  testWidgets('the dismiss button fires onDismiss', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build());

    await tester.tap(find.byKey(const Key('feedback_comment_dismiss')));

    expect(h.dismissed, 1);
  });

  testWidgets('initialText prefills the field', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build(initialText: 'wrong file'));

    expect(find.text('wrong file'), findsOneWidget);
    expect(_sendEnabled(tester), isTrue);
  });

  testWidgets('the limit counts Unicode scalar values', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build());
    // A family emoji is 7 scalar values (3 people + 2 joiners... plus
    // variation); use a 2-scalar flag so 1000 flags are exactly 2000 scalars.
    const flag = '\u{1F1E9}\u{1F1EA}';
    expect(flag.runes.length, 2);

    await tester.enterText(find.byKey(_input), flag * 1000);
    await tester.pump();
    expect(
      tester
          .widget<TextField>(find.byKey(_input))
          .controller!
          .text
          .runes
          .length,
      2000,
    );

    await tester.enterText(find.byKey(_input), '${flag * 1000}x');
    await tester.pump();
    expect(
      tester
          .widget<TextField>(find.byKey(_input))
          .controller!
          .text
          .runes
          .length,
      2000,
      reason: 'one more scalar is rejected',
    );
  });

  testWidgets('a custom maxLength is honoured', (tester) async {
    final h = _Harness();
    await tester.pumpWidget(h.build(maxLength: 5));

    await tester.enterText(find.byKey(_input), 'abcdefgh');
    await tester.pump();

    expect(find.text('abcde'), findsOneWidget);
  });
}
