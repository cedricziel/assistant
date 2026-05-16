// Tests for ContextTokenStorage — the adapter that bridges fresh_dio's
// TokenStorage<OAuthCredentials> contract to our ContextRepository.
//
// The adapter is scoped to a specific context ID at construction. read() is
// hot-path (called by fresh_dio on every request to attach the bearer), so it
// returns from an in-memory cache. write()/delete() update the cache AND
// persist through ContextRepository so the new credentials survive restart.

import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/context_token_storage.dart';
import 'package:assistant_app/features/auth/oauth_credentials.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';

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

OAuthCredentials _creds({
  String accessToken = 'access-1',
  String refreshToken = 'refresh-1',
  DateTime? expiresAt,
}) {
  return OAuthCredentials(
    accessToken: accessToken,
    refreshToken: refreshToken,
    expiresAt: expiresAt ?? DateTime.utc(2030, 1, 1),
    clientId: 'client-1',
  );
}

AssistantContext _ctx({String id = 'ctx-1', OAuthCredentials? oauth}) {
  return AssistantContext(
    id: id,
    name: 'Work',
    serverUrl: 'https://work.example.com',
    authMode: AuthMode.oauth2,
    oauthCredentials: oauth,
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

  group('ContextTokenStorage', () {
    test(
      'read() returns the initial credentials supplied at construction',
      () async {
        final initial = _creds(accessToken: 'initial');
        final storage = ContextTokenStorage(
          contextId: 'ctx-1',
          repository: repo,
          initial: initial,
        );

        expect(await storage.read(), initial);
      },
    );

    test('read() returns null when no initial credentials given', () async {
      final storage = ContextTokenStorage(contextId: 'ctx-1', repository: repo);

      expect(await storage.read(), isNull);
    });

    test('write() persists new credentials to the repository', () async {
      await repo.saveContext(_ctx(oauth: _creds(accessToken: 'old')));
      final storage = ContextTokenStorage(
        contextId: 'ctx-1',
        repository: repo,
        initial: _creds(accessToken: 'old'),
      );

      final fresh = _creds(accessToken: 'fresh', refreshToken: 'fresh-r');
      await storage.write(fresh);

      // In-memory cache reflects the new token.
      expect(await storage.read(), fresh);

      // Repository was updated.
      final loaded = await repo.loadContexts();
      expect(loaded.single.oauthCredentials?.accessToken, 'fresh');
      expect(loaded.single.oauthCredentials?.refreshToken, 'fresh-r');
    });

    test('delete() clears credentials in cache and repository', () async {
      await repo.saveContext(_ctx(oauth: _creds(accessToken: 'old')));
      final storage = ContextTokenStorage(
        contextId: 'ctx-1',
        repository: repo,
        initial: _creds(accessToken: 'old'),
      );

      await storage.delete();

      expect(await storage.read(), isNull);
      final loaded = await repo.loadContexts();
      expect(loaded.single.oauthCredentials, isNull);
    });

    test(
      'write() is a no-op on persistence when the context has been deleted',
      () async {
        // No context with id 'ctx-1' exists in the repository.
        final storage = ContextTokenStorage(
          contextId: 'ctx-1',
          repository: repo,
        );

        final fresh = _creds(accessToken: 'fresh');
        // Must not throw.
        await storage.write(fresh);

        // Cache still reflects the write (fresh_dio will retry against an empty
        // repository on next read — that's the deactivated-context case).
        expect(await storage.read(), fresh);
        expect(await repo.loadContexts(), isEmpty);
      },
    );
  });
}
