import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_list_tile.dart';

Widget _harness(Widget child) {
  return MaterialApp(home: Material(child: child));
}

void main() {
  group('AdaptiveListTile', () {
    testWidgets('renders CupertinoListTile on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(const AdaptiveListTile(title: Text('Hello'))),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoListTile), findsOneWidget);
      expect(find.byType(ListTile), findsNothing);
      expect(find.text('Hello'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders ListTile on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(const AdaptiveListTile(title: Text('Hello'))),
      );
      await tester.pumpAndSettle();

      expect(find.byType(ListTile), findsOneWidget);
      expect(find.byType(CupertinoListTile), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders ListTile on macOS (fallback)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        _harness(const AdaptiveListTile(title: Text('Hello'))),
      );
      await tester.pumpAndSettle();

      expect(find.byType(ListTile), findsOneWidget);
      expect(find.byType(CupertinoListTile), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('onTap fires on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      var tapped = false;

      await tester.pumpWidget(
        _harness(
          AdaptiveListTile(
            title: const Text('Tap me'),
            onTap: () => tapped = true,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byType(CupertinoListTile));
      expect(tapped, isTrue);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('onTap fires on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
      var tapped = false;

      await tester.pumpWidget(
        _harness(
          AdaptiveListTile(
            title: const Text('Tap me'),
            onTap: () => tapped = true,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byType(ListTile));
      expect(tapped, isTrue);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
