import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_nav_bar.dart';

void main() {
  group('AdaptiveNavBar', () {
    testWidgets('renders CupertinoNavigationBar on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        const CupertinoApp(
          home: CupertinoPageScaffold(
            navigationBar: AdaptiveNavBar(title: 'Test Title'),
            child: Text('body'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoNavigationBar), findsOneWidget);
      expect(find.byType(AppBar), findsNothing);
      expect(find.text('Test Title'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders AppBar on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            appBar: const AdaptiveNavBar(title: 'Test Title'),
            body: const Text('body'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(AppBar), findsOneWidget);
      expect(find.byType(CupertinoNavigationBar), findsNothing);
      expect(find.text('Test Title'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('passes actions to AppBar on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            appBar: AdaptiveNavBar(
              title: 'With Actions',
              actions: [
                IconButton(icon: const Icon(Icons.add), onPressed: () {}),
              ],
            ),
            body: const Text('body'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(Icons.add), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('passes trailing to CupertinoNavigationBar on iOS', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        CupertinoApp(
          home: CupertinoPageScaffold(
            navigationBar: AdaptiveNavBar(
              title: 'With Actions',
              actions: [
                CupertinoButton(
                  padding: EdgeInsets.zero,
                  onPressed: () {},
                  child: const Icon(CupertinoIcons.add),
                ),
              ],
            ),
            child: const Text('body'),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byIcon(CupertinoIcons.add), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
