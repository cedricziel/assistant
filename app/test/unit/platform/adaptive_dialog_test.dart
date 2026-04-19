import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_dialog.dart';

void main() {
  // 4.2.1 — Adaptive confirm dialog
  group('showAdaptiveConfirmDialog', () {
    testWidgets('CupertinoAlertDialog on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showAdaptiveConfirmDialog(
                context: context,
                title: 'Delete?',
                content: 'This cannot be undone.',
                confirmLabel: 'Delete',
                isDestructive: true,
              ),
              child: const Text('Trigger'),
            ),
          ),
        ),
      );

      await tester.tap(find.text('Trigger'));
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoAlertDialog), findsOneWidget);
      expect(find.text('Delete?'), findsOneWidget);
      expect(find.text('This cannot be undone.'), findsOneWidget);
      expect(find.text('Delete'), findsOneWidget);
      expect(find.text('Cancel'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('AlertDialog on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () => showAdaptiveConfirmDialog(
                context: context,
                title: 'Delete?',
                content: 'This cannot be undone.',
                confirmLabel: 'Delete',
                isDestructive: true,
              ),
              child: const Text('Trigger'),
            ),
          ),
        ),
      );

      await tester.tap(find.text('Trigger'));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.byType(CupertinoAlertDialog), findsNothing);
      expect(find.text('Delete?'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('returns true when confirmed', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      bool? result;

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showAdaptiveConfirmDialog(
                  context: context,
                  title: 'Delete?',
                  content: 'Sure?',
                  confirmLabel: 'Delete',
                );
              },
              child: const Text('Trigger'),
            ),
          ),
        ),
      );

      await tester.tap(find.text('Trigger'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(result, isTrue);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('returns false when cancelled', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      bool? result;

      await tester.pumpWidget(
        MaterialApp(
          home: Builder(
            builder: (context) => ElevatedButton(
              onPressed: () async {
                result = await showAdaptiveConfirmDialog(
                  context: context,
                  title: 'Delete?',
                  content: 'Sure?',
                  confirmLabel: 'Delete',
                );
              },
              child: const Text('Trigger'),
            ),
          ),
        ),
      );

      await tester.tap(find.text('Trigger'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(result, isFalse);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
