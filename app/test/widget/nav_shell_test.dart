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
            builder: (_, _) => const Scaffold(body: Text('Chat')),
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
      '5.1 NavigationBar has exactly 5 destinations at narrow width',
      (tester) async {
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
        await tester.pumpAndSettle();

        expect(find.byType(NavigationBar), findsOneWidget);
        expect(find.byType(NavigationDestination), findsNWidgets(5));
      },
    );

    testWidgets(
      '5.2 tapping "More" opens bottom sheet with all overflow destinations',
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

        // Bottom sheet should show all overflow destinations
        expect(find.text('Traces'), findsOneWidget);
        expect(find.text('Logs'), findsOneWidget);
        expect(find.text('Webhooks'), findsOneWidget);
        expect(find.text('Agents'), findsOneWidget);
        expect(find.text('Analytics'), findsOneWidget);
      },
    );

    testWidgets(
      '5.3 "More" destination is selected (index 4) when route is /traces',
      (tester) async {
        tester.view.physicalSize = const Size(375, 800);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/traces'));
        await tester.pumpAndSettle();

        final navBar =
            tester.widget<NavigationBar>(find.byType(NavigationBar));
        expect(
          navBar.selectedIndex,
          4,
          reason: '"More" (index 4) should be selected when on /traces',
        );
      },
    );
  });

  group('NavShell — desktop (>=768px)', () {
    testWidgets(
      '5.4 NavigationRail shows all 9 destinations with a divider between groups',
      (tester) async {
        tester.view.physicalSize = const Size(1280, 900);
        tester.view.devicePixelRatio = 1.0;
        addTearDown(tester.view.reset);

        await tester.pumpWidget(_buildNavShell(initialLocation: '/chat'));
        await tester.pumpAndSettle();

        expect(find.byType(NavigationRail), findsOneWidget);

        // All 9 destination labels must be visible in the rail
        for (final label in [
          'Chat',
          'Personas',
          'Skills',
          'Workflows',
          'Traces',
          'Logs',
          'Webhooks',
          'Agents',
          'Analytics',
        ]) {
          expect(
            find.descendant(
              of: find.byType(NavigationRail),
              matching: find.text(label),
            ),
            findsOneWidget,
            reason: '$label should appear in the navigation rail',
          );
        }

        // A Divider must be present between primary and overflow groups
        expect(
          find.descendant(
            of: find.byType(NavigationRail),
            matching: find.byType(Divider),
          ),
          findsWidgets,
        );
      },
    );
  });
}
