import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_action_sheet.dart';

void main() {
  group('showAdaptiveActionSheet', () {
    testWidgets('shows CupertinoActionSheet on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      late BuildContext capturedCtx;

      await tester.pumpWidget(
        MaterialApp(
          home: Material(
            child: Builder(
              builder: (ctx) {
                capturedCtx = ctx;
                return const SizedBox.shrink();
              },
            ),
          ),
        ),
      );

      showAdaptiveActionSheet<String>(
        context: capturedCtx,
        title: 'Pick one',
        actions: [
          AdaptiveActionSheetAction(
            label: 'A',
            onPressed: (ctx) => Navigator.of(ctx).pop('a'),
          ),
          AdaptiveActionSheetAction(
            label: 'B',
            onPressed: (ctx) => Navigator.of(ctx).pop('b'),
            isDestructive: true,
          ),
        ],
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoActionSheet), findsOneWidget);
      expect(find.text('Pick one'), findsOneWidget);
      expect(find.text('A'), findsOneWidget);
      expect(find.text('B'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('shows modal bottom sheet on Android', (tester) async {
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

      showAdaptiveActionSheet<String>(
        context: capturedCtx,
        title: 'Pick one',
        actions: [
          AdaptiveActionSheetAction(
            label: 'A',
            onPressed: (ctx) => Navigator.of(ctx).pop('a'),
          ),
        ],
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoActionSheet), findsNothing);
      expect(find.text('Pick one'), findsOneWidget);
      expect(find.text('A'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('returns selected value on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      late BuildContext capturedCtx;

      await tester.pumpWidget(
        MaterialApp(
          home: Material(
            child: Builder(
              builder: (ctx) {
                capturedCtx = ctx;
                return const SizedBox.shrink();
              },
            ),
          ),
        ),
      );

      final future = showAdaptiveActionSheet<String>(
        context: capturedCtx,
        title: 'Pick one',
        actions: [
          AdaptiveActionSheetAction(
            label: 'A',
            onPressed: (ctx) => Navigator.of(ctx).pop('a'),
          ),
        ],
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('A'));
      await tester.pumpAndSettle();
      expect(await future, 'a');

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
