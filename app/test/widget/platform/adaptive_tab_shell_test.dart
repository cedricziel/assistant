import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/shared/nav_shell.dart';

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

Widget _buildNavShell({required String initialLocation}) {
  final router = GoRouter(
    initialLocation: initialLocation,
    routes: [
      ShellRoute(
        builder: (ctx, state, child) => NavShell(child: child),
        routes: [
          GoRoute(
            path: '/chat',
            builder: (_, _) => const Scaffold(body: Text('Chat content')),
          ),
          GoRoute(
            path: '/contexts',
            builder: (_, _) => const Scaffold(body: Text('Contexts content')),
          ),
          GoRoute(
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('Skills content')),
          ),
          GoRoute(
            path: '/workflows',
            builder: (_, _) => const Scaffold(body: Text('Workflows content')),
          ),
          GoRoute(
            path: '/personas',
            builder: (_, _) => const Scaffold(body: Text('Personas content')),
          ),
          GoRoute(
            path: '/traces',
            builder: (_, _) => const Scaffold(body: Text('Traces content')),
          ),
          GoRoute(
            path: '/logs',
            builder: (_, _) => const Scaffold(body: Text('Logs content')),
          ),
          GoRoute(
            path: '/webhooks',
            builder: (_, _) => const Scaffold(body: Text('Webhooks content')),
          ),
          GoRoute(
            path: '/agents',
            builder: (_, _) => const Scaffold(body: Text('Agents content')),
          ),
          GoRoute(
            path: '/analytics',
            builder: (_, _) => const Scaffold(body: Text('Analytics content')),
          ),
          GoRoute(
            path: '/settings',
            builder: (_, _) => const Scaffold(body: Text('Settings content')),
          ),
        ],
      ),
    ],
  );
  return ProviderScope(
    overrides: [pwaInstallProvider.overrideWith(_FakePwaNotifier.new)],
    child: MaterialApp.router(routerConfig: router),
  );
}

void main() {
  group('NavShell — iOS compact (CupertinoTabBar)', () {
    testWidgets('2.1.1 renders CupertinoTabBar with 5 items on iOS at 375dp', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // On iOS compact, NavShell should render a CupertinoTabBar
      expect(find.byType(CupertinoTabBar), findsOneWidget);
      // No Material NavigationBar
      expect(find.byType(NavigationBar), findsNothing);

      // 4 primary (Chat, Contexts, Skills, Workflows) + More = 5
      final tabBar = tester.widget<CupertinoTabBar>(
        find.byType(CupertinoTabBar),
      );
      expect(
        tabBar.items.length,
        5,
        reason: 'Should have Chat, Contexts, Skills, Workflows, More',
      );

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets(
      '2.1.2 tapping More opens Cupertino popup with overflow destinations',
      (tester) async {
        debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
        await tester.pumpAndSettle();

        // Tap the "More" tab
        await tester.tap(find.text('More'));
        await tester.pumpAndSettle();

        // Should show a Cupertino-styled action sheet with overflow destinations
        expect(find.byType(CupertinoActionSheet), findsOneWidget);

        // All overflow destinations visible in the sheet
        expect(find.text('Personas'), findsOneWidget);
        expect(find.text('Traces'), findsOneWidget);
        expect(find.text('Logs'), findsOneWidget);
        expect(find.text('Webhooks'), findsOneWidget);
        expect(find.text('Agents'), findsOneWidget);
        expect(find.text('Analytics'), findsOneWidget);
        expect(find.text('Settings'), findsOneWidget);

        debugDefaultTargetPlatformOverride = null;
      },
    );

    testWidgets('2.1.2b tapping an overflow destination navigates there', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // Tap More → Traces
      await tester.tap(find.text('More'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Traces'));
      await tester.pumpAndSettle();

      expect(find.text('Traces content'), findsOneWidget);

      debugDefaultTargetPlatformOverride = null;
    });
  });

  group('NavShell — iOS regular width (sidebar)', () {
    testWidgets('2.2.1 sidebar with all destinations on iOS at 1024dp', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.iOS;

      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // No CupertinoTabBar on wide iPad
      expect(find.byType(CupertinoTabBar), findsNothing);
      // No Material NavigationRail either
      expect(find.byType(NavigationRail), findsNothing);

      // All destinations visible in the sidebar list
      for (final label in [
        'Chat',
        'Contexts',
        'Skills',
        'Workflows',
        'Personas',
        'Traces',
        'Logs',
        'Webhooks',
        'Agents',
        'Analytics',
        'Settings',
      ]) {
        expect(
          find.text(label),
          findsOneWidget,
          reason: '$label should appear in the sidebar',
        );
      }

      debugDefaultTargetPlatformOverride = null;
    });
  });

  group('NavShell — Material non-regression', () {
    testWidgets('2.3.1 Material NavigationBar on macOS at 375dp', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      expect(find.byType(NavigationBar), findsOneWidget);
      expect(find.byType(CupertinoTabBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });

    testWidgets('2.3.1 Material collapsible sidebar on macOS at 1024dp', (
      tester,
    ) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;

      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // The Material wide layout now uses a custom collapsible sidebar
      // instead of NavigationRail. Verify sidebar toggle and labels.
      expect(find.byTooltip('Collapse sidebar'), findsOneWidget);
      expect(find.text('Chat'), findsOneWidget);
      expect(find.byType(CupertinoTabBar), findsNothing);

      debugDefaultTargetPlatformOverride = null;
    });
  });
}
