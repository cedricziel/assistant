import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/features/settings/settings_screen.dart';

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

Widget _harness(Widget screen) {
  return ProviderScope(
    overrides: [
      apiClientProvider.overrideWithValue(null),
      pwaInstallProvider.overrideWith(_FakePwaNotifier.new),
    ],
    child: MaterialApp(home: Material(child: screen)),
  );
}

void main() {
  // 4.1.1 — Adaptive switches (SwitchListTile.adaptive)
  group('Adaptive switches — Settings', () {
    testWidgets('3 switches render on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      // iOS uses CupertinoSwitch inside CupertinoListTile.
      expect(
        find.byType(CupertinoSwitch),
        findsNWidgets(3),
        reason: 'Settings should show 3 CupertinoSwitch toggles on iOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('3 switches render on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      expect(
        find.byType(SwitchListTile),
        findsNWidgets(3),
        reason: 'Settings should show 3 switch tiles on macOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });
  });

  // 4.1.3 — Adaptive spinners (CircularProgressIndicator.adaptive)
  group('Adaptive spinners', () {
    testWidgets('CupertinoActivityIndicator on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      // Standalone adaptive spinner — verifies the replacement works.
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: Center(child: CircularProgressIndicator.adaptive()),
          ),
        ),
      );

      expect(
        find.byType(CupertinoActivityIndicator),
        findsOneWidget,
        reason:
            'CircularProgressIndicator.adaptive should render CupertinoActivityIndicator on iOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('Material spinner on Android', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.android;

      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(
            body: Center(child: CircularProgressIndicator.adaptive()),
          ),
        ),
      );

      expect(
        find.byType(CupertinoActivityIndicator),
        findsNothing,
        reason:
            'CircularProgressIndicator.adaptive should NOT render CupertinoActivityIndicator on Android',
      );

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
