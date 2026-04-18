// Smoke test: the app can be pumped without crashing.

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/main.dart';

void main() {
  testWidgets('App smoke test — renders without crash', (
    WidgetTester tester,
  ) async {
    // Wrap in ProviderScope (same as production main.dart).
    await tester.pumpWidget(const ProviderScope(child: AssistantApp()));
    // Allow async providers to settle.
    await tester.pump(const Duration(milliseconds: 100));
    // The app should be showing something (not crash).
    expect(find.byType(AssistantApp), findsOneWidget);
  });
}
