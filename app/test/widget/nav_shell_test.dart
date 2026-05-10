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
            builder: (_, _) => const Scaffold(body: Text('Chat page')),
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
            path: '/contexts',
            builder: (_, _) => const Scaffold(body: Text('Personas content')),
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
  group('NavShell — mobile (<768px)', () {
    testWidgets(
      '5.1 NavigationBar has exactly 4 destinations at narrow width',
      (tester) async {
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
        await tester.pumpAndSettle();

        expect(find.byType(NavigationBar), findsOneWidget);
        // 3 primary (Chat, Skills, Workflows) + More
        expect(find.byType(NavigationDestination), findsNWidgets(4));
      },
    );

    testWidgets(
      '5.2 tapping "More" opens bottom sheet with Contexts and overflow destinations',
      (tester) async {
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
        await tester.pumpAndSettle();

        // "More" must be in the navigation bar
        expect(
          find.descendant(
            of: find.byType(NavigationBar),
            matching: find.text('More'),
          ),
          findsOneWidget,
        );

        await tester.tap(find.text('More'));
        await tester.pumpAndSettle();

        // Contexts switcher must appear first in the sheet
        expect(find.text('Contexts'), findsOneWidget);
        // Overflow destinations also present
        expect(find.text('Traces'), findsOneWidget);
        expect(find.text('Logs'), findsOneWidget);
        expect(find.text('Webhooks'), findsOneWidget);
        expect(find.text('Agents'), findsOneWidget);
        expect(find.text('Analytics'), findsOneWidget);
        // Settings is at the bottom, separated by a divider
        expect(find.text('Settings'), findsOneWidget);
      },
    );

    testWidgets('5.3 tapping Contexts in More sheet navigates to /contexts', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(375, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      await tester.tap(find.text('More'));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Contexts'));
      await tester.pumpAndSettle();

      // /contexts stub renders this text (see _buildNavShell above)
      expect(find.text('Personas content'), findsOneWidget);
      // "More" button is selected (index 3) when on an overflow route
      final navBar = tester.widget<NavigationBar>(find.byType(NavigationBar));
      expect(
        navBar.selectedIndex,
        3,
        reason: '"More" (index 3) should be selected when on /contexts',
      );
    });

    testWidgets(
      '5.4 "More" destination is selected (index 3) when route is /traces',
      (tester) async {
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/traces'));
        await tester.pumpAndSettle();

        final navBar = tester.widget<NavigationBar>(find.byType(NavigationBar));
        expect(
          navBar.selectedIndex,
          3,
          reason: '"More" (index 3) should be selected when on /traces',
        );
      },
    );
  });

  group('NavShell — desktop (>=768px)', () {
    testWidgets('5.4 sidebar shows all destination labels and toggle button', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // All destination labels visible in the expanded sidebar
      for (final label in [
        'Chat',
        'Skills',
        'Workflows',
        'Personas',
        'Traces',
        'Logs',
        'Webhooks',
        'Agents',
        'Analytics',
      ]) {
        expect(
          find.text(label),
          findsOneWidget,
          reason: '$label should appear in the sidebar',
        );
      }

      // Contexts and Settings are pinned below the scrollable area
      for (final tooltip in ['Contexts', 'Settings']) {
        expect(
          find.text(tooltip),
          findsOneWidget,
          reason: '$tooltip should be visible in the sidebar',
        );
      }

      // A toggle button to collapse the sidebar must be present
      expect(
        find.byTooltip('Collapse sidebar'),
        findsOneWidget,
        reason: 'Collapse toggle should be visible when sidebar is expanded',
      );
    });

    testWidgets('5.5 tapping toggle collapses sidebar to icon-only mode', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // Sidebar starts expanded — labels visible
      expect(find.text('Chat'), findsOneWidget);
      expect(find.text('Skills'), findsOneWidget);

      // Collapse
      await tester.tap(find.byTooltip('Collapse sidebar'));
      await tester.pumpAndSettle();

      // After collapse, the expand toggle should appear
      expect(
        find.byTooltip('Expand sidebar'),
        findsOneWidget,
        reason: 'Expand toggle should be visible when sidebar is collapsed',
      );

      // Labels should be hidden in collapsed mode — only tooltips remain
      // (text labels are not rendered, icons are shown with tooltips)
      expect(
        find.text('Skills'),
        findsNothing,
        reason: 'Destination labels should be hidden when collapsed',
      );
    });

    testWidgets('5.6 navigation works in collapsed sidebar mode', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // Collapse the sidebar
      await tester.tap(find.byTooltip('Collapse sidebar'));
      await tester.pumpAndSettle();

      // Tap the Skills icon via its tooltip
      await tester.tap(find.byTooltip('Skills'));
      await tester.pumpAndSettle();

      // Should navigate to skills content
      expect(find.text('Skills content'), findsOneWidget);
    });

    testWidgets('5.7 tapping toggle again expands sidebar back', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // Collapse
      await tester.tap(find.byTooltip('Collapse sidebar'));
      await tester.pumpAndSettle();

      // Expand
      await tester.tap(find.byTooltip('Expand sidebar'));
      await tester.pumpAndSettle();

      // Labels should be visible again
      expect(find.text('Chat'), findsOneWidget);
      expect(find.text('Skills'), findsOneWidget);
      expect(
        find.byTooltip('Collapse sidebar'),
        findsOneWidget,
        reason: 'Collapse toggle should reappear after expanding',
      );
    });

    // 5.8 — every platform: Contexts AND Logout button visible.
    //
    // kIsWeb is a compile-time constant; in tests it is always false (non-web).
    // Pre-#688 the logout button was gated `if (kIsWeb)` and therefore absent
    // on native — that gate is now gone. Both Contexts and Logout coexist.
    testWidgets('5.8 desktop shows both Contexts and Logout button', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1024, 768);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      // Contexts must be visible in the sidebar trailing section.
      expect(
        find.text('Contexts'),
        findsOneWidget,
        reason: 'Contexts switcher should be present',
      );

      // Logout button must now appear on every platform.
      expect(
        find.byTooltip('Log out'),
        findsOneWidget,
        reason: 'Logout button should be present on every platform',
      );
    });
  });
}
