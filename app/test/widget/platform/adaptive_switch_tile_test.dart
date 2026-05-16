import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_switch_tile.dart';

Widget _harness(Widget child) {
  return MaterialApp(home: Material(child: child));
}

void main() {
  group('AdaptiveSwitchTile', () {
    testWidgets('renders CupertinoListTile + CupertinoSwitch on iOS', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveSwitchTile(
            title: const Text('Toggle'),
            value: false,
            onChanged: (_) {},
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoListTile), findsOneWidget);
      expect(find.byType(CupertinoSwitch), findsOneWidget);
      expect(find.byType(SwitchListTile), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders SwitchListTile on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(
          AdaptiveSwitchTile(
            title: const Text('Toggle'),
            value: false,
            onChanged: (_) {},
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(SwitchListTile), findsOneWidget);
      expect(find.byType(CupertinoSwitch), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('toggling fires onChanged on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      bool? captured;

      await tester.pumpWidget(
        _harness(
          AdaptiveSwitchTile(
            title: const Text('Toggle'),
            value: false,
            onChanged: (v) => captured = v,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byType(CupertinoSwitch));
      await tester.pumpAndSettle();
      expect(captured, isTrue);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders subtitle on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(
          AdaptiveSwitchTile(
            title: const Text('Toggle'),
            subtitle: const Text('Description'),
            value: false,
            onChanged: (_) {},
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Description'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
