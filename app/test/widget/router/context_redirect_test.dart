import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/contexts/screens/context_switcher_screen.dart';
import 'package:assistant_app/router/app_router.dart';
import 'package:assistant_app/shared/loading_screen.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

Future<ContextRepository> makeRepo({String? activeContextId}) async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  final repo = ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
  if (activeContextId != null) {
    await repo.setActiveContextId(activeContextId);
  }
  return repo;
}

Widget buildApp(ContextRepository repo) {
  return ProviderScope(
    overrides: [
      contextRepositoryProvider.overrideWithValue(repo),
      capabilitiesProvider.overrideWith(
        (ref) async => ServerCapabilities.disabled,
      ),
      // Prevent feature notifiers from firing real Dio HTTP requests.
      // These are routing tests — API responses are irrelevant here.
      apiClientProvider.overrideWithValue(null),
    ],
    child: Consumer(
      builder: (context, ref, child) {
        final router = ref.watch(routerProvider);
        return MaterialApp.router(routerConfig: router);
      },
    ),
  );
}

void main() {
  group('Loading scaffold', () {
    testWidgets(
      'shows LoadingScreen while activeContextProvider is AsyncLoading',
      (tester) async {
        // Override activeContextProvider to stay in AsyncLoading indefinitely.
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              activeContextProvider.overrideWith(
                () => _AlwaysLoadingContextNotifier(),
              ),
            ],
            child: Consumer(
              builder: (context, ref, child) {
                final router = ref.watch(routerProvider);
                return MaterialApp.router(routerConfig: router);
              },
            ),
          ),
        );

        // First frame: provider is AsyncLoading → router shows /loading.
        await tester.pump();

        expect(find.byType(LoadingScreen), findsOneWidget);
        expect(find.byType(CircularProgressIndicator), findsOneWidget);
        expect(find.text('Starting\u2026'), findsOneWidget);
        // Navigation chrome (NavShell) must not be present.
        expect(find.byKey(const Key('nav_shell')), findsNothing);
      },
    );

    testWidgets('settled with active context → loading screen absent', (
      tester,
    ) async {
      final repo = await makeRepo();
      final ctx = AssistantContext(
        id: 'ctx-1',
        name: 'Work',
        serverUrl: 'https://work.example.com',
        createdAt: DateTime.utc(2024, 1, 1),
      );
      await repo.saveContext(ctx);
      await repo.setActiveContextId(ctx.id);

      await tester.pumpWidget(buildApp(repo));
      // 1st pump: provider loads, redirect fires, navigation starts.
      await tester.pump(const Duration(milliseconds: 500));
      // 2nd pump: page-transition animation completes.
      await tester.pump(const Duration(milliseconds: 500));

      // Provider settled with an active context → not on loading screen.
      expect(find.byType(LoadingScreen), findsNothing);
    });

    testWidgets(
      'settled with no context → loading screen absent, context switcher shown',
      (tester) async {
        final repo = await makeRepo();

        await tester.pumpWidget(buildApp(repo));
        // Let the provider load, redirect fire, and page transition complete.
        await tester.pumpAndSettle();

        // Provider settled with null context → context switcher, no loading.
        expect(find.byType(LoadingScreen), findsNothing);
        expect(find.byType(ContextSwitcherScreen), findsOneWidget);
      },
    );
  });

  group('Router redirect', () {
    testWidgets('redirects to /contexts when no active context', (
      tester,
    ) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildApp(repo));
      // Pump several frames: loading → provider settles → redirect → render.
      for (var i = 0; i < 5; i++) {
        await tester.pump();
      }

      // Should be on the ContextSwitcherScreen (AppBar title "Contexts").
      expect(find.text('Contexts'), findsWidgets);
    });

    testWidgets(
      'stays on /contexts when already on it with no active context',
      (tester) async {
        final repo = await makeRepo();
        await tester.pumpWidget(buildApp(repo));
        for (var i = 0; i < 5; i++) {
          await tester.pump();
        }

        // Still on Contexts screen — no redirect loop.
        expect(find.text('Contexts'), findsWidgets);
      },
    );

    testWidgets(
      'with active context, /contexts is accessible (no auto-redirect to /chat)',
      (tester) async {
        final repo = await makeRepo();
        final ctx = AssistantContext(
          id: 'ctx-1',
          name: 'Work',
          serverUrl: 'https://work.example.com',
          createdAt: DateTime.utc(2024, 1, 1),
        );
        await repo.saveContext(ctx);
        await repo.setActiveContextId(ctx.id);

        await tester.pumpWidget(buildApp(repo));
        for (var i = 0; i < 5; i++) {
          await tester.pump();
        }

        // Router should NOT redirect /contexts → /chat when a context is active.
        // The initial location is /chat (no redirect needed), so navigate to
        // /contexts explicitly and confirm it stays there.
        final innerContainer = ProviderScope.containerOf(
          tester.element(find.byType(Consumer)),
        );
        final router = innerContainer.read(routerProvider);
        router.go('/contexts');
        for (var i = 0; i < 5; i++) {
          await tester.pump();
        }

        expect(find.byType(ContextSwitcherScreen), findsOneWidget);
      },
    );
  });
}

/// An [ActiveContextNotifier] that never resolves — stays in [AsyncLoading].
class _AlwaysLoadingContextNotifier extends ActiveContextNotifier {
  @override
  Future<AssistantContext?> build() async {
    // Return a Future that never completes.
    await Completer<AssistantContext?>().future;
    return null;
  }
}
