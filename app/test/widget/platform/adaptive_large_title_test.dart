import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/features/settings/settings_screen.dart';
import 'package:assistant_app/features/skills/skills_screen.dart';
import 'package:assistant_app/features/personas/personas_screen.dart';
import 'package:assistant_app/features/traces/traces_screen.dart';
import 'package:assistant_app/features/logs/logs_screen.dart';
import 'package:assistant_app/features/contexts/screens/context_switcher_screen.dart';
import 'package:assistant_app/features/webhooks/webhooks_screen.dart';
import 'package:assistant_app/features/agents/agents_screen.dart';
import 'package:assistant_app/features/analytics/analytics_screen.dart';
import 'package:assistant_app/features/workflows/workflows_screen.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

/// Pump a screen with common provider overrides that prevent API calls.
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
  // 3.2.1 — Settings screen
  group('Large title — Settings', () {
    testWidgets('CupertinoSliverNavigationBar on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      expect(find.byType(CupertinoSliverNavigationBar), findsOneWidget);
      expect(find.text('Settings'), findsWidgets); // largeTitle + collapsed

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('AppBar on macOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const SettingsScreen()));
      await tester.pumpAndSettle();

      expect(find.byType(AppBar), findsOneWidget);
      expect(find.byType(CupertinoSliverNavigationBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });
  });

  // 3.2.3 — Parameterized test for remaining list screens
  final screenMatrix = <String, Widget>{
    'Skills': const SkillsScreen(),
    'Personas': const PersonasScreen(),
    'Traces': const TracesScreen(),
    'Logs': const LogsScreen(),
    'Contexts': const ContextSwitcherScreen(),
    'Webhooks': const WebhooksScreen(),
    'Agents': const AgentsScreen(),
    'Analytics': const AnalyticsScreen(),
    'Workflows': const WorkflowsScreen(),
  };

  for (final entry in screenMatrix.entries) {
    group('Large title — ${entry.key}', () {
      testWidgets('CupertinoSliverNavigationBar on iOS', (tester) async {
        debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
        SharedPreferences.setMockInitialValues({});
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_harness(entry.value));
        await tester.pumpAndSettle();

        expect(
          find.byType(CupertinoSliverNavigationBar),
          findsOneWidget,
          reason:
              '${entry.key} should show CupertinoSliverNavigationBar on iOS',
        );

        debugDefaultTargetPlatformOverride = null;
      });

      testWidgets('AppBar on macOS', (tester) async {
        debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
        SharedPreferences.setMockInitialValues({});
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_harness(entry.value));
        await tester.pumpAndSettle();

        expect(
          find.byType(AppBar),
          findsOneWidget,
          reason: '${entry.key} should show AppBar on macOS',
        );
        expect(
          find.byType(CupertinoSliverNavigationBar),
          findsNothing,
          reason:
              '${entry.key} should not show CupertinoSliverNavigationBar on macOS',
        );

        debugDefaultTargetPlatformOverride = null;
      });
    });
  }

  // 3.3 — Chat screen chrome (compact nav bar, no large title)
  group('Chat screen chrome', () {
    testWidgets('3.3.1 CupertinoNavigationBar on iOS', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const ChatScreen()));
      await tester.pump();

      expect(find.byType(CupertinoNavigationBar), findsOneWidget);
      expect(find.byType(AppBar), findsNothing);
      // No large title on chat
      expect(find.byType(CupertinoSliverNavigationBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('3.3.3 AppBar on macOS (non-regression)', (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      SharedPreferences.setMockInitialValues({});
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_harness(const ChatScreen()));
      await tester.pump();

      expect(find.byType(AppBar), findsOneWidget);
      expect(find.byType(CupertinoNavigationBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
