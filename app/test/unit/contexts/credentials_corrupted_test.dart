// Tests that ContextRepository.loadContexts surfaces secure-storage
// decrypt failures via AssistantContext.credentialsCorrupted instead of
// silently dropping the token.
//
// Spec: openspec/changes/web-session-resilience/specs/web-session-resilience/spec.md
//   "Decrypt failure during context load is detected and surfaced"

import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/auth/oauth_credentials.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';

class _ThrowingSecureStorage implements SecureKeyValueStorage {
  _ThrowingSecureStorage({this.throwOnRead = false});

  final bool throwOnRead;
  final Map<String, String> _data = {};

  @override
  Future<String?> read({required String key}) async {
    if (throwOnRead) {
      throw StateError('simulated decrypt failure for key=$key');
    }
    return _data[key];
  }

  @override
  Future<void> write({required String key, required String value}) async {
    _data[key] = value;
  }

  @override
  Future<void> delete({required String key}) async => _data.remove(key);
}

class _PartialThrowingSecureStorage implements SecureKeyValueStorage {
  _PartialThrowingSecureStorage(this.throwOnPrefix);

  final String throwOnPrefix;
  final Map<String, String> _data = {};

  @override
  Future<String?> read({required String key}) async {
    if (key.startsWith(throwOnPrefix)) {
      throw StateError('simulated decrypt failure for key=$key');
    }
    return _data[key];
  }

  @override
  Future<void> write({required String key, required String value}) async {
    _data[key] = value;
  }

  @override
  Future<void> delete({required String key}) async => _data.remove(key);
}

Future<ContextRepository> _makeRepo(SecureKeyValueStorage storage) async {
  SharedPreferences.setMockInitialValues({});
  final prefs = await SharedPreferences.getInstance();
  return ContextRepository(prefs: prefs, secureStorage: storage);
}

AssistantContext _ctx({String id = 'ctx-1'}) {
  return AssistantContext(
    id: id,
    name: 'Work',
    serverUrl: 'https://work.example.com',
    authMode: AuthMode.oauth2,
    createdAt: DateTime.utc(2024, 1, 1),
  );
}

OAuthCredentials _creds() => OAuthCredentials(
  accessToken: 'access',
  refreshToken: 'refresh',
  clientId: 'client',
  expiresAt: DateTime.utc(2099, 1, 1),
);

void main() {
  test('healthy load — credentialsCorrupted stays false', () async {
    final storage = _ThrowingSecureStorage();
    final repo = await _makeRepo(storage);

    await repo.saveContext(_ctx().copyWith(oauthCredentials: _creds()));

    final loaded = await repo.loadContexts();

    expect(loaded, hasLength(1));
    expect(loaded.first.credentialsCorrupted, isFalse);
    expect(loaded.first.oauthCredentials, isNotNull);
  });

  test(
    'decrypt failure flips credentialsCorrupted, preserves metadata',
    () async {
      // First persist a context with valid storage.
      final goodStorage = _ThrowingSecureStorage();
      final goodRepo = await _makeRepo(goodStorage);
      await goodRepo.saveContext(_ctx().copyWith(oauthCredentials: _creds()));

      // Now reload through a storage that throws on every read — simulating
      // the IndexedDB key wipe that flutter_secure_storage hits on web.
      final brokenStorage = _ThrowingSecureStorage(throwOnRead: true);
      // Persist the same metadata in SharedPreferences for the broken repo.
      SharedPreferences.setMockInitialValues({
        'assistant_contexts': goodRepo.dumpMetadataJson()!,
      });
      final brokenPrefs = await SharedPreferences.getInstance();
      final brokenRepo = ContextRepository(
        prefs: brokenPrefs,
        secureStorage: brokenStorage,
      );

      final loaded = await brokenRepo.loadContexts();

      expect(loaded, hasLength(1));
      final ctx = loaded.first;
      expect(ctx.credentialsCorrupted, isTrue, reason: 'flag must be set');
      expect(ctx.oauthCredentials, isNull);
      expect(ctx.authToken, isNull);
      expect(ctx.id, 'ctx-1');
      expect(ctx.name, 'Work');
      expect(ctx.serverUrl, 'https://work.example.com');
      expect(ctx.authMode, AuthMode.oauth2);
    },
  );

  test(
    'partial corruption — token OK but oauth read throws still flags',
    () async {
      // Persist via healthy storage, then mount a partial-throwing storage that
      // only fails on the oauth-credentials key. Even partial corruption taints
      // the whole session.
      final goodStorage = _ThrowingSecureStorage();
      final goodRepo = await _makeRepo(goodStorage);
      await goodRepo.saveContext(_ctx().copyWith(oauthCredentials: _creds()));

      final partialStorage = _PartialThrowingSecureStorage(
        'assistant_context_oauth_',
      );
      SharedPreferences.setMockInitialValues({
        'assistant_contexts': goodRepo.dumpMetadataJson()!,
      });
      final brokenPrefs = await SharedPreferences.getInstance();
      final partialRepo = ContextRepository(
        prefs: brokenPrefs,
        secureStorage: partialStorage,
      );

      final loaded = await partialRepo.loadContexts();
      expect(loaded.first.credentialsCorrupted, isTrue);
    },
  );
}
