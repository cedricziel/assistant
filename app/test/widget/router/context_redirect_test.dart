import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/contexts/screens/context_switcher_screen.dart';
import 'package:assistant_app/router/app_router.dart';

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
    overrides: [contextRepositoryProvider.overrideWithValue(repo)],
    child: Consumer(
      builder: (context, ref, child) {
        final router = ref.watch(routerProvider);
        return MaterialApp.router(routerConfig: router);
      },
    ),
  );
}

void main() {
  group('Router redirect', () {
    testWidgets('redirects to /contexts when no active context', (
      tester,
    ) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildApp(repo));
      await tester.pumpAndSettle();

      // Should be on the ContextSwitcherScreen (AppBar title "Contexts").
      expect(find.text('Contexts'), findsWidgets);
    });

    testWidgets(
      'stays on /contexts when already on it with no active context',
      (tester) async {
        final repo = await makeRepo();
        await tester.pumpWidget(buildApp(repo));
        await tester.pumpAndSettle();

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
        await tester.pumpAndSettle();

        // Router should NOT redirect /contexts → /chat when a context is active.
        // The initial location is /chat (no redirect needed), so navigate to
        // /contexts explicitly and confirm it stays there.
        final innerContainer = ProviderScope.containerOf(
          tester.element(find.byType(Consumer)),
        );
        final router = innerContainer.read(routerProvider);
        router.go('/contexts');
        await tester.pumpAndSettle();

        expect(find.byType(ContextSwitcherScreen), findsOneWidget);
      },
    );
  });
}
