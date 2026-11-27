import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

Future<void> _pump(
  WidgetTester tester, {
  String message = 'Connection lost',
  VoidCallback? onDismiss,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: ThreadErrorBanner(message: message, onDismiss: onDismiss ?? () {}),
    ),
  ),
);

void main() {
  group('ThreadErrorBanner', () {
    testWidgets('shows the message and a warning icon', (tester) async {
      await _pump(tester, message: 'Rate limited');

      expect(find.text('Rate limited'), findsOneWidget);
      expect(find.byIcon(Icons.warning_amber_outlined), findsOneWidget);
    });

    testWidgets('the close button fires onDismiss', (tester) async {
      var dismissed = 0;
      await _pump(tester, onDismiss: () => dismissed++);

      await tester.tap(find.byIcon(Icons.close));
      await tester.pump();

      expect(dismissed, 1);
    });

    testWidgets('uses the error container colours', (tester) async {
      await _pump(tester);

      final scheme = Theme.of(tester.element(find.byType(Scaffold)))
          .colorScheme;
      final material = tester.widget<Material>(
        find
            .descendant(
              of: find.byType(ThreadErrorBanner),
              matching: find.byType(Material),
            )
            .first,
      );
      expect(material.color, scheme.errorContainer);
    });

    testWidgets('the dismiss button is labelled "Dismiss error"', (
      tester,
    ) async {
      final handle = tester.ensureSemantics();
      await _pump(tester);

      expect(
        tester.widget<IconButton>(find.byType(IconButton)).tooltip,
        'Dismiss error',
      );
      handle.dispose();
    });

    testWidgets('the banner is a live region', (tester) async {
      final handle = tester.ensureSemantics();
      await _pump(tester, message: 'Rate limited');

      final node = tester.getSemantics(find.byType(ThreadErrorBanner));
      expect(
        node.flagsCollection.isLiveRegion,
        isTrue,
        reason: 'an error that appears is announced by assistive tech',
      );
      handle.dispose();
    });
  });
}
