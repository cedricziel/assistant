import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/shared/nav_shell.dart';

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

Widget _buildShell({required String initialLocation}) {
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
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('Skills content')),
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
  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  group('Top-leading sidebar toggle (Material wide)', () {
    testWidgets('iPad-landscape viewport shows Hide navigation toggle', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1180, 820);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      expect(
        find.byTooltip('Hide navigation'),
        findsOneWidget,
        reason: 'Top-leading sidebar toggle should be visible when expanded',
      );
    });

    testWidgets('tapping the top-leading toggle collapses the sidebar', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1180, 820);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      await tester.tap(find.byTooltip('Hide navigation'));
      await tester.pumpAndSettle();

      expect(find.byTooltip('Show navigation'), findsOneWidget);
      expect(find.byTooltip('Hide navigation'), findsNothing);
    });

    testWidgets('tapping again expands the sidebar', (tester) async {
      tester.view.physicalSize = const Size(1180, 820);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      await tester.tap(find.byTooltip('Hide navigation'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Show navigation'));
      await tester.pumpAndSettle();

      expect(find.byTooltip('Hide navigation'), findsOneWidget);
    });

    testWidgets('toggle is not rendered at compact width', (tester) async {
      tester.view.physicalSize = const Size(420, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      await tester.pumpWidget(_buildShell(initialLocation: '/chat'));
      await tester.pumpAndSettle();

      expect(find.byTooltip('Hide navigation'), findsNothing);
      expect(find.byTooltip('Show navigation'), findsNothing);
    });
  });
}
