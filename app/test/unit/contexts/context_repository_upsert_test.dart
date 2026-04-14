import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  final Map<String, String> _data = {};
  @override
  Future<String?> read({required String key}) async => _data[key];
  @override
  Future<void> write({required String key, required String value}) async =>
      _data[key] = value;
  @override
  Future<void> delete({required String key}) async => _data.remove(key);
}

void main() {
  late ContextRepository repo;

  setUp(() async {
    SharedPreferences.setMockInitialValues({});
    final prefs = await SharedPreferences.getInstance();
    repo = ContextRepository(prefs: prefs, secureStorage: _FakeSecureStorage());
  });

  AssistantContext makeCtx({
    String id = 'ctx-1',
    String name = 'local',
    String serverUrl = 'http://localhost:8080',
    String? authToken,
  }) {
    return AssistantContext(
      id: id,
      name: name,
      serverUrl: serverUrl,
      authToken: authToken,
      createdAt: DateTime.utc(2024, 1, 1),
    );
  }

  group('upsertContextByUrl', () {
    test('inserts a new context when no matching URL exists', () async {
      final ctx = makeCtx();
      final saved = await repo.upsertContextByUrl(ctx);

      expect(saved.id, ctx.id);
      final all = await repo.loadContexts();
      expect(all.length, 1);
      expect(all.single.serverUrl, 'http://localhost:8080');
    });

    test(
      'updates token when a context with the same URL already exists',
      () async {
        final first = makeCtx(authToken: 'old-token');
        await repo.upsertContextByUrl(first);

        final updated = makeCtx(id: 'different-id', authToken: 'new-token');
        final saved = await repo.upsertContextByUrl(updated);

        // ID is preserved from the first insertion.
        expect(saved.id, first.id);

        final all = await repo.loadContexts();
        expect(all.length, 1, reason: 'no duplicate should be created');
        expect(all.single.authToken, 'new-token');
      },
    );

    test('preserves createdAt from original when updating', () async {
      final first = makeCtx();
      await repo.upsertContextByUrl(first);

      final updated = makeCtx(authToken: 'tok');
      final saved = await repo.upsertContextByUrl(updated);

      expect(saved.createdAt, first.createdAt);
    });

    test('two different URLs create two separate contexts', () async {
      await repo.upsertContextByUrl(
        makeCtx(id: 'a', serverUrl: 'http://localhost:8080'),
      );
      await repo.upsertContextByUrl(
        makeCtx(id: 'b', serverUrl: 'http://localhost:9090'),
      );

      final all = await repo.loadContexts();
      expect(all.length, 2);
    });
  });
}
