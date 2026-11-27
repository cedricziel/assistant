import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(WidgetTester tester, Widget body) async {
  await tester.pumpWidget(MaterialApp(home: Scaffold(body: body)));
}

void main() {
  group('CommandBody', () {
    testWidgets('renders the command name and the acknowledgement', (
      tester,
    ) async {
      await _pump(
        tester,
        const CommandBody(name: 'model', ack: 'Model set to gpt-4o'),
      );

      expect(find.text('/model'), findsOneWidget);
      expect(find.text('Model set to gpt-4o'), findsOneWidget);
    });

    testWidgets('uses system-event styling, not a chat bubble', (tester) async {
      await _pump(
        tester,
        const CommandBody(name: 'new', ack: 'New conversation started'),
      );

      expect(find.byKey(CommandBody.tileKey), findsOneWidget);
      expect(find.byType(Row), findsOneWidget);
    });

    testWidgets('renders the terminal icon', (tester) async {
      await _pump(
        tester,
        const CommandBody(name: 'compact', ack: 'History compacted'),
      );

      expect(find.byIcon(Icons.terminal), findsOneWidget);
    });

    testWidgets('a long acknowledgement ellipsizes instead of overflowing', (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: 200,
              child: CommandBody(name: 'model', ack: 'x' * 200),
            ),
          ),
        ),
      );

      expect(tester.takeException(), isNull);
      final ack = tester.widget<Text>(find.text('x' * 200));
      expect(ack.overflow, TextOverflow.ellipsis);
    });
  });
}
