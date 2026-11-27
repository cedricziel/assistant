import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  group('TypingIndicator', () {
    testWidgets('renders three dots', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(home: Scaffold(body: TypingIndicator())),
      );

      expect(
        find.descendant(
          of: find.byType(TypingIndicator),
          matching: find.byType(DecoratedBox),
        ),
        findsNWidgets(3),
      );
    });

    testWidgets('animates, and draws still dots when animations are off', (
      tester,
    ) async {
      await tester.pumpWidget(
        const MaterialApp(home: Scaffold(body: TypingIndicator())),
      );
      expect(find.byType(Opacity), findsNWidgets(3));

      await tester.pumpWidget(
        const MediaQuery(
          data: MediaQueryData(disableAnimations: true),
          child: MaterialApp(home: Scaffold(body: TypingIndicator())),
        ),
      );
      expect(find.byType(Opacity), findsNothing);
    });

    testWidgets('reduced motion schedules no frames and stays still', (
      tester,
    ) async {
      await tester.pumpWidget(
        const MediaQuery(
          data: MediaQueryData(disableAnimations: true),
          child: MaterialApp(home: Scaffold(body: TypingIndicator())),
        ),
      );
      await tester.pump();

      expect(tester.binding.hasScheduledFrame, isFalse);
      expect(tester.binding.transientCallbackCount, 0);

      final before = tester
          .widgetList<DecoratedBox>(find.byType(DecoratedBox))
          .map((box) => (box.decoration as BoxDecoration).color)
          .toList();

      await tester.pump(const Duration(seconds: 1));

      final after = tester
          .widgetList<DecoratedBox>(find.byType(DecoratedBox))
          .map((box) => (box.decoration as BoxDecoration).color)
          .toList();

      expect(after, before);
      expect(tester.binding.hasScheduledFrame, isFalse);
      expect(tester.binding.transientCallbackCount, 0);
    });
  });
}
