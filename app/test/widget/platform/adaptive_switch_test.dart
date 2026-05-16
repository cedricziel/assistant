import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/platform/adaptive_switch.dart';

Widget _harness(Widget child) {
  return MaterialApp(
    home: Material(child: Center(child: child)),
  );
}

void main() {
  group('AdaptiveSwitch', () {
    testWidgets('renders CupertinoSwitch on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      await tester.pumpWidget(
        _harness(AdaptiveSwitch(value: false, onChanged: (_) {})),
      );
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoSwitch), findsOneWidget);
      expect(find.byType(Switch), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Switch on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(AdaptiveSwitch(value: false, onChanged: (_) {})),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Switch), findsOneWidget);
      expect(find.byType(CupertinoSwitch), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('renders Switch on macOS (fallback)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      await tester.pumpWidget(
        _harness(AdaptiveSwitch(value: false, onChanged: (_) {})),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Switch), findsOneWidget);
      expect(find.byType(CupertinoSwitch), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('toggling fires onChanged on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      bool? captured;

      await tester.pumpWidget(
        _harness(AdaptiveSwitch(value: false, onChanged: (v) => captured = v)),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byType(CupertinoSwitch));
      await tester.pumpAndSettle();
      expect(captured, isTrue);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('null onChanged renders disabled on Material', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        _harness(const AdaptiveSwitch(value: true, onChanged: null)),
      );
      await tester.pumpAndSettle();

      final swt = tester.widget<Switch>(find.byType(Switch));
      expect(swt.onChanged, isNull);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
