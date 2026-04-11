import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';

/// In-memory secure storage for tests.
class _FakeSecureStorage implements SecureKeyValueStorage {
  final Map<String, String> _data = {};

  @override
  Future<String?> read({required String key}) async => _data[key];

  @override
  Future<void> write({required String key, required String value}) async {
    _data[key] = value;
  }

  @override
  Future<void> delete({required String key}) async => _data.remove(key);
}

ProviderContainer makeContainer(ContextRepository repo) {
  return ProviderContainer(
    overrides: [contextRepositoryProvider.overrideWithValue(repo)],
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
  late SharedPreferences prefs;
  late _FakeSecureStorage secure;
  late ContextRepository repo;

  setUp(() async {
    SharedPreferences.setMockInitialValues({});
    prefs = await SharedPreferences.getInstance();
    secure = _FakeSecureStorage();
    repo = ContextRepository(prefs: prefs, secureStorage: secure);
  });

  group('activeContextProvider', () {
    test('is null when no context has been saved', () async {
      final container = makeContainer(repo);
      addTearDown(container.dispose);

      final active = await container.read(activeContextProvider.future);
      expect(active, isNull);
    });

    test('is null when contexts exist but none is active', () async {
      await repo.saveContext(makeCtx());

      final container = makeContainer(repo);
      addTearDown(container.dispose);

      final active = await container.read(activeContextProvider.future);
      expect(active, isNull);
    });

    test('returns the active context after activate()', () async {
      final ctx = makeCtx();
      await repo.saveContext(ctx);

      final container = makeContainer(repo);
      addTearDown(container.dispose);

      // Ensure contexts are loaded.
      await container.read(contextsProvider.future);

      await container.read(activeContextProvider.notifier).activate(ctx);
      final active = await container.read(activeContextProvider.future);
      expect(active?.id, ctx.id);
    });

    test('deactivate() sets active context to null', () async {
      final ctx = makeCtx();
      await repo.saveContext(ctx);
      await repo.setActiveContextId(ctx.id);

      final container = makeContainer(repo);
      addTearDown(container.dispose);

      // Ensure contexts are loaded.
      await container.read(contextsProvider.future);

      await container.read(activeContextProvider.notifier).deactivate();
      final active = await container.read(activeContextProvider.future);
      expect(active, isNull);
    });

    test('returns null after the active context is deleted', () async {
      final ctx = makeCtx();
      await repo.saveContext(ctx);
      await repo.setActiveContextId(ctx.id);

      final container = makeContainer(repo);
      addTearDown(container.dispose);

      await container.read(contextsProvider.notifier).deleteContext(ctx.id);
      // Wait for reactive rebuild.
      await Future<void>.delayed(const Duration(milliseconds: 10));
      final active = await container.read(activeContextProvider.future);
      expect(active, isNull);
    });

    test('activate() persists the active context ID', () async {
      final ctx = makeCtx();
      await repo.saveContext(ctx);

      final container = makeContainer(repo);
      addTearDown(container.dispose);

      await container.read(contextsProvider.future);
      await container.read(activeContextProvider.notifier).activate(ctx);

      expect(repo.getActiveContextId(), ctx.id);
    });
  });
}
