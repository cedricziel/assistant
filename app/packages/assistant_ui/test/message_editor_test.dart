import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'support/golden.dart';

Finder _field() => find.byType(TextField);
Finder _send() => find.widgetWithText(FilledButton, 'Send');
Finder _cancel() => find.widgetWithText(TextButton, 'Cancel');

bool _sendEnabled(WidgetTester tester) =>
    tester.widget<FilledButton>(_send()).onPressed != null;

void main() {
  group('MessageEditor', () {
    testWidgets('disables Send while the text is unchanged', (tester) async {
      await pumpGolden(
        tester,
        MessageEditor(initialText: 'hello', onSubmit: (_) {}, onCancel: () {}),
      );

      expect(find.text('hello'), findsOneWidget);
      expect(_sendEnabled(tester), isFalse);
    });

    testWidgets('disables Send when the text is blank', (tester) async {
      await pumpGolden(
        tester,
        MessageEditor(initialText: 'hello', onSubmit: (_) {}, onCancel: () {}),
      );

      await tester.enterText(_field(), '   \n ');
      await tester.pump();

      expect(_sendEnabled(tester), isFalse);
    });

    testWidgets('disables Send when only whitespace was added', (tester) async {
      await pumpGolden(
        tester,
        MessageEditor(initialText: 'hello', onSubmit: (_) {}, onCancel: () {}),
      );

      await tester.enterText(_field(), 'hello  ');
      await tester.pump();

      expect(_sendEnabled(tester), isFalse);
    });

    testWidgets('submits the trimmed text', (tester) async {
      final submitted = <String>[];
      await pumpGolden(
        tester,
        MessageEditor(
          initialText: 'hello',
          onSubmit: submitted.add,
          onCancel: () {},
        ),
      );

      await tester.enterText(_field(), '  hello world \n');
      await tester.pump();
      expect(_sendEnabled(tester), isTrue);
      await tester.tap(_send());

      expect(submitted, ['hello world']);
    });

    testWidgets('Cancel fires onCancel', (tester) async {
      var cancelled = 0;
      await pumpGolden(
        tester,
        MessageEditor(
          initialText: 'hello',
          onSubmit: (_) {},
          onCancel: () => cancelled++,
        ),
      );

      await tester.tap(_cancel());

      expect(cancelled, 1);
    });

    testWidgets('Escape fires onCancel', (tester) async {
      var cancelled = 0;
      await pumpGolden(
        tester,
        MessageEditor(
          initialText: 'hello',
          onSubmit: (_) {},
          onCancel: () => cancelled++,
        ),
      );

      await tester.tap(_field());
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pump();

      expect(cancelled, 1);
    });

    testWidgets('renders attachment widgets beneath the field', (tester) async {
      await pumpGolden(
        tester,
        MessageEditor(
          initialText: 'hello',
          onSubmit: (_) {},
          onCancel: () {},
          attachments: const [
            Chip(label: Text('report.pdf')),
            Chip(label: Text('photo.png')),
          ],
        ),
      );

      expect(find.text('report.pdf'), findsOneWidget);
      expect(find.text('photo.png'), findsOneWidget);
      expect(
        tester.getTopLeft(find.text('report.pdf')).dy,
        greaterThan(tester.getBottomLeft(_field()).dy - 1),
        reason: 'attachments sit below the text field',
      );
    });
  });
}
