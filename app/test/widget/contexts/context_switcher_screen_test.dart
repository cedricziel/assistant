import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/contexts/screens/context_switcher_screen.dart';

/// Minimal in-memory secure storage for tests.
class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

Future<ContextRepository> makeRepo([Map<String, Object>? prefs]) async {
  SharedPreferences.setMockInitialValues(prefs ?? {});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

Widget buildScreen(ContextRepository repo) {
  return ProviderScope(
    overrides: [contextRepositoryProvider.overrideWithValue(repo)],
    child: MaterialApp.router(
      routerConfig: GoRouter(
        initialLocation: '/contexts',
        routes: [
          GoRoute(
            path: '/contexts',
            builder: (ctx, s) => const ContextSwitcherScreen(),
          ),
          GoRoute(path: '/chat', builder: (ctx, s) => const Scaffold()),
        ],
      ),
    ),
  );
}

AssistantContext makeCtx({
  String id = 'ctx-1',
  String name = 'Work',
  String serverUrl = 'https://work.example.com',
}) {
  return AssistantContext(
    id: id,
    name: name,
    serverUrl: serverUrl,
    createdAt: DateTime.utc(2024, 1, 1),
  );
}

void main() {
  group('ContextSwitcherScreen', () {
    testWidgets('shows empty state when no contexts exist', (tester) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      expect(find.text('No contexts yet. Tap + to add one.'), findsOneWidget);
    });

    testWidgets('shows context list when contexts exist', (tester) async {
      final repo = await makeRepo();
      await repo.saveContext(makeCtx());
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      expect(find.text('Work'), findsOneWidget);
      expect(find.text('https://work.example.com'), findsOneWidget);
    });

    testWidgets('shows active indicator on active context', (tester) async {
      final repo = await makeRepo();
      final ctx = makeCtx();
      await repo.saveContext(ctx);
      await repo.setActiveContextId(ctx.id);
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      // Active context shows a check_circle icon.
      expect(find.byIcon(Icons.check_circle), findsOneWidget);
    });

    testWidgets('FAB opens create dialog', (tester) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.byType(FloatingActionButton));
      await tester.pumpAndSettle();

      expect(find.text('New Context'), findsOneWidget);
      expect(find.text('Name'), findsOneWidget);
      expect(find.text('URL'), findsOneWidget);
    });

    testWidgets('dialog Cancel dismisses without saving', (tester) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.byType(FloatingActionButton));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(find.text('New Context'), findsNothing);
      expect(await repo.loadContexts(), isEmpty);
    });

    testWidgets('dialog validates empty name', (tester) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.byType(FloatingActionButton));
      await tester.pumpAndSettle();

      // Tap Save without entering anything.
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(find.text('Name is required'), findsOneWidget);
    });

    testWidgets('dialog validates invalid URL', (tester) async {
      final repo = await makeRepo();
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.byType(FloatingActionButton));
      await tester.pumpAndSettle();

      await tester.enterText(
        find.widgetWithText(TextFormField, 'Name'),
        'Test',
      );
      await tester.enterText(
        find.widgetWithText(TextFormField, 'URL'),
        'not-a-url',
      );
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(find.textContaining('valid URL'), findsOneWidget);
    });

    testWidgets('delete icon shows confirmation dialog', (tester) async {
      final repo = await makeRepo();
      await repo.saveContext(makeCtx());
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      expect(find.text('Delete context?'), findsOneWidget);
    });

    testWidgets('confirming delete removes context from list', (tester) async {
      final repo = await makeRepo();
      await repo.saveContext(makeCtx());
      await tester.pumpWidget(buildScreen(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.text('Work'), findsNothing);
      expect(await repo.loadContexts(), isEmpty);
    });
  });
}
