import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(WidgetTester tester, ThreadEmptyState state) =>
    tester.pumpWidget(MaterialApp(home: Scaffold(body: state)));

void main() {
  group('ThreadEmptyState', () {
    testWidgets('defaults to the "Start a conversation" copy', (tester) async {
      await _pump(tester, const ThreadEmptyState());

      expect(find.text('Start a conversation'), findsOneWidget);
      expect(find.text('Type a message below to begin.'), findsOneWidget);
    });

    testWidgets('renders a custom title and subtitle', (tester) async {
      await _pump(
        tester,
        const ThreadEmptyState(title: 'Hello', subtitle: 'Ask me anything.'),
      );

      expect(find.text('Hello'), findsOneWidget);
      expect(find.text('Ask me anything.'), findsOneWidget);
      expect(find.text('Start a conversation'), findsNothing);
    });

    testWidgets('renders the starters widget below the subtitle', (
      tester,
    ) async {
      await _pump(
        tester,
        const ThreadEmptyState(starters: Text('starters here')),
      );

      final subtitle = tester.getBottomLeft(
        find.text('Type a message below to begin.'),
      );
      final starters = tester.getTopLeft(find.text('starters here'));
      expect(starters.dy, greaterThanOrEqualTo(subtitle.dy));
    });

    testWidgets('renders the starter prompts element in the slot', (
      tester,
    ) async {
      String? picked;
      await _pump(
        tester,
        ThreadEmptyState(
          starters: StarterPrompts(
            prompts: const ['Plan my week'],
            onSelected: (p) => picked = p,
          ),
        ),
      );

      await tester.tap(find.byKey(const Key('starter_prompt_0')));

      expect(picked, 'Plan my week');
    });
  });
}
