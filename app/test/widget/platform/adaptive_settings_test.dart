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
  // 4.4.1 — CupertinoListSection.insetGrouped on iOS
  group('Settings — grouped list', () {
    testWidgets('CupertinoListSection on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      expect(
        find.byType(CupertinoListSection),
        findsOneWidget,
        reason: 'Settings should use CupertinoListSection.insetGrouped on iOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('No CupertinoListSection on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      expect(
        find.byType(CupertinoListSection),
        findsNothing,
        reason: 'Settings should NOT use CupertinoListSection on macOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
