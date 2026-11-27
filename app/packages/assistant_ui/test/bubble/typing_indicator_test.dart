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
  });
}
