import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

/// Pumps the widget with semantics enabled, so `bySemanticsLabel` can find
/// the toggle buttons.
void _test(String name, Future<void> Function(WidgetTester tester) body) =>
    testWidgets(name, (tester) async {
      final handle = tester.ensureSemantics();
      try {
        await tester.pumpWidget(_app());
        await body(tester);
      } finally {
        handle.dispose();
      }
    });

Widget _app() => const MaterialApp(
  home: Scaffold(
    body: CompactExpandable(
      summary: Text('exit 2 · 14 lines stderr'),
      expanded: Text('full terminal view'),
    ),
  ),
);

void main() {
  _test('starts on the summary with a show button', (tester) async {
    expect(find.text('exit 2 · 14 lines stderr'), findsOneWidget);
    expect(find.text('full terminal view'), findsNothing);
    expect(find.bySemanticsLabel('Show full result'), findsOneWidget);
  });

  _test('tapping the show button swaps in the full view', (tester) async {
    await tester.tap(find.bySemanticsLabel('Show full result'));
    await tester.pump();

    expect(find.text('full terminal view'), findsOneWidget);
    expect(find.text('exit 2 · 14 lines stderr'), findsNothing);
    expect(find.bySemanticsLabel('Hide full result'), findsOneWidget);
    expect(find.bySemanticsLabel('Show full result'), findsNothing);
  });

  testWidgets('initiallyExpanded starts on the full view', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: CompactExpandable(
            initiallyExpanded: true,
            summary: Text('exit 2 · 14 lines stderr'),
            expanded: Text('full terminal view'),
          ),
        ),
      ),
    );

    expect(find.text('full terminal view'), findsOneWidget);
    expect(find.text('exit 2 · 14 lines stderr'), findsNothing);
  });

  _test('tapping the summary text also expands', (tester) async {
    await tester.tap(find.text('exit 2 · 14 lines stderr'));
    await tester.pump();

    expect(find.text('full terminal view'), findsOneWidget);
  });

  _test('tapping hide restores the summary', (tester) async {
    await tester.tap(find.bySemanticsLabel('Show full result'));
    await tester.pump();

    await tester.tap(find.bySemanticsLabel('Hide full result'));
    await tester.pump();

    expect(find.text('exit 2 · 14 lines stderr'), findsOneWidget);
    expect(find.text('full terminal view'), findsNothing);
    expect(find.bySemanticsLabel('Show full result'), findsOneWidget);
  });

  _test('both buttons are exposed as semantic buttons', (tester) async {
    expect(
      tester.getSemantics(find.bySemanticsLabel('Show full result')),
      isSemantics(
        label: 'Show full result',
        isButton: true,
        hasTapAction: true,
      ),
    );

    await tester.tap(find.bySemanticsLabel('Show full result'));
    await tester.pump();

    expect(
      tester.getSemantics(find.bySemanticsLabel('Hide full result')),
      isSemantics(
        label: 'Hide full result',
        isButton: true,
        hasTapAction: true,
      ),
    );
  });
}
