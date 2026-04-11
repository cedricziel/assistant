import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';

/// In-memory secure storage for tests.
class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

Future<ContextRepository> makeRepo() async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

void main() {
  // The macOS tray menu is built by TrayService which requires the
  // tray_manager platform channel — not available in the flutter test
  // environment.  These tests validate the provider-level behaviour that
  // TrayService reads from: activeContextProvider and contextsProvider.

  group('Tray menu provider behaviour', () {
    test(
      'activeContextProvider returns null when no context is active',
      () async {
        final repo = await makeRepo();
        final container = ProviderContainer(
          overrides: [contextRepositoryProvider.overrideWithValue(repo)],
        );
        addTearDown(container.dispose);

        final active = await container.read(activeContextProvider.future);
        expect(active, isNull);
      },
    );

    test('activeContextProvider returns context after activate()', () async {
      final repo = await makeRepo();
      final ctx = AssistantContext(
        id: 'ctx-1',
        name: 'Work',
        serverUrl: 'https://work.example.com',
        createdAt: DateTime.utc(2024, 1, 1),
      );
      await repo.saveContext(ctx);

      final container = ProviderContainer(
        overrides: [contextRepositoryProvider.overrideWithValue(repo)],
      );
      addTearDown(container.dispose);

      await container.read(contextsProvider.future);
      await container.read(activeContextProvider.notifier).activate(ctx);

      final active = await container.read(activeContextProvider.future);
      expect(active?.name, 'Work');
    });

    test('contextsProvider returns all saved contexts', () async {
      final repo = await makeRepo();
      await repo.saveContext(
        AssistantContext(
          id: 'ctx-1',
          name: 'Work',
          serverUrl: 'https://work.example.com',
          createdAt: DateTime.utc(2024, 1, 1),
        ),
      );
      await repo.saveContext(
        AssistantContext(
          id: 'ctx-2',
          name: 'Personal',
          serverUrl: 'http://localhost:8080',
          createdAt: DateTime.utc(2024, 2, 1),
        ),
      );

      final container = ProviderContainer(
        overrides: [contextRepositoryProvider.overrideWithValue(repo)],
      );
      addTearDown(container.dispose);

      final contexts = await container.read(contextsProvider.future);
      expect(contexts.length, 2);
      expect(contexts.map((c) => c.name), containsAll(['Work', 'Personal']));
    });
  });
}
