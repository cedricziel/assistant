import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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
  // 4.5.1 — Haptic feedback on switch toggle (iOS only)
  group('Haptic feedback', () {
    testWidgets('lightImpact on switch toggle (iOS)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      final hapticCalls = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'HapticFeedback.vibrate') {
            hapticCalls.add(call.arguments as String);
          }
          return null;
        },
      );
      addTearDown(() {
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        );
      });

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      // Tap the first switch tile (CupertinoSwitch on iOS).
      final switches = find.byType(CupertinoSwitch);
      expect(switches, findsNWidgets(3));
      await tester.tap(switches.first);
      await tester.pumpAndSettle();

      expect(
        hapticCalls,
        contains('HapticFeedbackType.lightImpact'),
        reason: 'Switch toggle should trigger light haptic on iOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('no haptic on switch toggle (macOS)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      final hapticCalls = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'HapticFeedback.vibrate') {
            hapticCalls.add(call.arguments as String);
          }
          return null;
        },
      );
      addTearDown(() {
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        );
      });

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      // Tap the first switch tile on macOS.
      final switches = find.byType(SwitchListTile);
      expect(switches, findsNWidgets(3));
      await tester.tap(switches.first);
      await tester.pumpAndSettle();

      expect(
        hapticCalls.where((c) => c == 'HapticFeedbackType.lightImpact'),
        isEmpty,
        reason: 'Switch toggle should NOT trigger haptic on macOS',
      );

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
