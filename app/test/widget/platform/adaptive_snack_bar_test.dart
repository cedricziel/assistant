import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_snack_bar.dart';

void main() {
  group('showAdaptiveSnackBar', () {
    testWidgets('shows Material SnackBar on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      late BuildContext capturedCtx;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (ctx) {
                capturedCtx = ctx;
                return const SizedBox.shrink();
              },
            ),
          ),
        ),
      );

      showAdaptiveSnackBar(capturedCtx, 'Hello world');
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Hello world'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('shows Cupertino-style overlay on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      late BuildContext capturedCtx;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (ctx) {
                capturedCtx = ctx;
                return const SizedBox.shrink();
              },
            ),
          ),
        ),
      );

      showAdaptiveSnackBar(capturedCtx, 'Hello iOS');
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Hello iOS'), findsOneWidget);
      // Should NOT use a Material SnackBar on iOS — we render via overlay.
      expect(find.byType(SnackBar), findsNothing);

      // Let the timer expire before the test ends.
      await tester.pump(const Duration(seconds: 3));

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
