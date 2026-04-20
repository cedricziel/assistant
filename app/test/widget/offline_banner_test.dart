import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/api/connectivity_provider.dart';
import 'package:assistant_app/features/pwa/pwa_provider.dart';
import 'package:assistant_app/shared/nav_shell.dart';

class _FakePwaNotifier extends PwaInstallNotifier {
  @override
  bool build() => false;
}

Widget _buildApp({
  required Stream<List<ConnectivityResult>> connectivityStream,
}) {
  final router = GoRouter(
    initialLocation: '/chat',
    routes: [
      ShellRoute(
        builder: (ctx, state, child) => NavShell(child: child),
        routes: [
          GoRoute(
            path: '/chat',
            builder: (_, _) => const Scaffold(body: Text('Chat content')),
          ),
          GoRoute(
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('Skills content')),
          ),
          GoRoute(
            path: '/workflows',
            builder: (_, _) => const Scaffold(body: Text('Workflows content')),
          ),
        ],
      ),
    ],
  );
  return ProviderScope(
    overrides: [
      pwaInstallProvider.overrideWith(_FakePwaNotifier.new),
      connectivityProvider.overrideWith((ref) => connectivityStream),
    ],
    child: MaterialApp.router(routerConfig: router),
  );
}

void main() {
  group('Offline banner', () {
    testWidgets('shows banner when connectivity is none', (tester) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      final controller = StreamController<List<ConnectivityResult>>();
      addTearDown(controller.close);

      await tester.pumpWidget(_buildApp(connectivityStream: controller.stream));
      await tester.pumpAndSettle();

      // Initially loading → no banner (optimistic online).
      expect(find.text('No internet connection'), findsNothing);

      // Emit offline.
      controller.add([ConnectivityResult.none]);
      await tester.pumpAndSettle();

      expect(find.text('No internet connection'), findsOneWidget);
      expect(find.byIcon(Icons.wifi_off), findsOneWidget);
    });

    testWidgets('hides banner when connectivity is restored', (tester) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      final controller = StreamController<List<ConnectivityResult>>();
      addTearDown(controller.close);

      await tester.pumpWidget(_buildApp(connectivityStream: controller.stream));
      await tester.pumpAndSettle();

      // Go offline.
      controller.add([ConnectivityResult.none]);
      await tester.pumpAndSettle();
      expect(find.text('No internet connection'), findsOneWidget);

      // Come back online.
      controller.add([ConnectivityResult.wifi]);
      await tester.pumpAndSettle();
      expect(find.text('No internet connection'), findsNothing);
    });

    testWidgets('child content remains visible when offline', (tester) async {
      tester.view.physicalSize = const Size(1280, 900);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.reset);

      final controller = StreamController<List<ConnectivityResult>>();
      addTearDown(controller.close);

      await tester.pumpWidget(_buildApp(connectivityStream: controller.stream));
      await tester.pumpAndSettle();

      controller.add([ConnectivityResult.none]);
      await tester.pumpAndSettle();

      // Banner is shown AND content is still visible.
      expect(find.text('No internet connection'), findsOneWidget);
      expect(find.text('Chat content'), findsOneWidget);
    });
  });
}
