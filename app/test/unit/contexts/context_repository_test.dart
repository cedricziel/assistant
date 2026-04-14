import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';

/// In-memory fake for [SecureKeyValueStorage] used in unit tests.
class FakeSecureStorage implements SecureKeyValueStorage {
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

/// Fake that always throws on [read] — simulates the web crypto key loss
/// that can happen after a hard reload.
class ThrowingSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async =>
      throw Exception('crypto key unavailable');

  @override
  Future<void> write({required String key, required String value}) async {}

  @override
  Future<void> delete({required String key}) async {}
}

void main() {
  late SharedPreferences prefs;
  late FakeSecureStorage secure;
  late ContextRepository repo;

  setUp(() async {
    SharedPreferences.setMockInitialValues({});
    prefs = await SharedPreferences.getInstance();
    secure = FakeSecureStorage();
    repo = ContextRepository(prefs: prefs, secureStorage: secure);
  });

  AssistantContext makeCtx({
    String id = 'ctx-1',
    String name = 'Work',
    String serverUrl = 'https://work.example.com',
    String? authToken,
    DateTime? createdAt,
  }) {
    return AssistantContext(
      id: id,
      name: name,
      serverUrl: serverUrl,
      authToken: authToken,
      createdAt: createdAt ?? DateTime.utc(2024, 1, 1),
    );
  }

  group('loadContexts', () {
    test('returns empty list when nothing saved', () async {
      final result = await repo.loadContexts();
      expect(result, isEmpty);
    });

    test('returns saved contexts sorted by createdAt ascending', () async {
      final later = makeCtx(
        id: 'ctx-2',
        name: 'Personal',
        serverUrl: 'http://localhost',
        createdAt: DateTime.utc(2024, 2, 1),
      );
      final earlier = makeCtx(createdAt: DateTime.utc(2024, 1, 1));
      await repo.saveContext(later);
      await repo.saveContext(earlier);

      final result = await repo.loadContexts();
      expect(result.length, 2);
      expect(result.first.id, 'ctx-1'); // earlier createdAt
      expect(result.last.id, 'ctx-2');
    });

    test('restores auth token from secure storage', () async {
      final ctx = makeCtx(authToken: 'my-secret-token');
      await repo.saveContext(ctx);

      final result = await repo.loadContexts();
      expect(result.single.authToken, 'my-secret-token');
    });

    test('returns null authToken when no token stored', () async {
      final ctx = makeCtx(); // no token
      await repo.saveContext(ctx);

      final result = await repo.loadContexts();
      expect(result.single.authToken, isNull);
    });

    test('returns context without token when secure storage throws', () async {
      // Simulate the web hard-reload scenario where the crypto key is
      // unavailable and FlutterSecureStorage throws on read.
      final ctx = makeCtx(authToken: 'secret');
      // Write metadata via a working repo, then read via the throwing one.
      await repo.saveContext(ctx);

      final throwingRepo = ContextRepository(
        prefs: prefs,
        secureStorage: ThrowingSecureStorage(),
      );
      // Must not throw; context should be returned without its token.
      final result = await throwingRepo.loadContexts();
      expect(result.length, 1);
      expect(result.single.id, ctx.id);
      expect(result.single.authToken, isNull);
    });
  });

  group('saveContext', () {
    test('inserts a new context', () async {
      await repo.saveContext(makeCtx());
      final result = await repo.loadContexts();
      expect(result.length, 1);
      expect(result.single.name, 'Work');
    });

    test('updates existing context by id', () async {
      await repo.saveContext(makeCtx());
      await repo.saveContext(makeCtx(name: 'Work Updated'));

      final result = await repo.loadContexts();
      expect(result.length, 1);
      expect(result.single.name, 'Work Updated');
    });

    test('saves token to secure storage', () async {
      await repo.saveContext(makeCtx(authToken: 'tok'));
      expect(secure._data.values, contains('tok'));
    });

    test('clears token from secure storage when token removed', () async {
      await repo.saveContext(makeCtx(authToken: 'tok'));
      await repo.saveContext(makeCtx(authToken: null));
      expect(secure._data.values, isNot(contains('tok')));
    });
  });

  group('deleteContext', () {
    test('removes context from metadata', () async {
      await repo.saveContext(makeCtx());
      await repo.deleteContext('ctx-1');
      expect(await repo.loadContexts(), isEmpty);
    });

    test('removes token from secure storage', () async {
      await repo.saveContext(makeCtx(authToken: 'tok'));
      await repo.deleteContext('ctx-1');
      expect(secure._data, isEmpty);
    });

    test('clears active context ID when active is deleted', () async {
      await repo.saveContext(makeCtx());
      await repo.setActiveContextId('ctx-1');
      expect(repo.getActiveContextId(), 'ctx-1');

      await repo.deleteContext('ctx-1');
      expect(repo.getActiveContextId(), isNull);
    });

    test(
      'does not clear active ID when a different context is deleted',
      () async {
        await repo.saveContext(makeCtx(id: 'ctx-1'));
        await repo.saveContext(
          makeCtx(id: 'ctx-2', name: 'Personal', serverUrl: 'http://localhost'),
        );
        await repo.setActiveContextId('ctx-1');

        await repo.deleteContext('ctx-2');
        expect(repo.getActiveContextId(), 'ctx-1');
      },
    );
  });

  group('getActiveContextId / setActiveContextId', () {
    test('returns null initially', () {
      expect(repo.getActiveContextId(), isNull);
    });

    test('persists an id', () async {
      await repo.setActiveContextId('ctx-1');
      expect(repo.getActiveContextId(), 'ctx-1');
    });

    test('clears id when null is passed', () async {
      await repo.setActiveContextId('ctx-1');
      await repo.setActiveContextId(null);
      expect(repo.getActiveContextId(), isNull);
    });
  });
}
