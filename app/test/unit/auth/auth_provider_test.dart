import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/auth/auth_provider.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';

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

Future<ContextRepository> _makeRepo() async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

ProviderContainer _makeContainer(ContextRepository repo) {
  return ProviderContainer(
    overrides: [contextRepositoryProvider.overrideWithValue(repo)],
  );
}

void main() {
  group('OAuthLoginState', () {
    test('OAuthLoginIdle is an OAuthLoginState', () {
      const state = OAuthLoginIdle();
      expect(state, isA<OAuthLoginState>());
    });

    test('OAuthLoginLoading has default message', () {
      const state = OAuthLoginLoading();
      expect(state.message, 'Connecting...');
    });

    test('OAuthLoginLoading accepts custom message', () {
      const state = OAuthLoginLoading(message: 'Checking server...');
      expect(state.message, 'Checking server...');
    });

    test('OAuthLoginSuccess carries context', () {
      final ctx = AssistantContext.create(
        name: 'test',
        serverUrl: 'http://localhost',
      );
      final state = OAuthLoginSuccess(ctx);
      expect(state.context.name, 'test');
    });

    test('OAuthLoginError carries message', () {
      const state = OAuthLoginError('bad credentials');
      expect(state.message, 'bad credentials');
    });
  });

  group('OAuthLoginNotifier', () {
    test('initial state is OAuthLoginIdle', () async {
      final repo = await _makeRepo();
      final container = _makeContainer(repo);
      addTearDown(container.dispose);

      final state = container.read(oauthLoginProvider);
      expect(state, isA<OAuthLoginIdle>());
    });

    group('legacyLogin', () {
      test('creates context and transitions to Success', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);
        await notifier.legacyLogin(serverUrl: 'http://localhost:8080');

        final state = container.read(oauthLoginProvider);
        expect(state, isA<OAuthLoginSuccess>());
        final success = state as OAuthLoginSuccess;
        expect(success.context.serverUrl, 'http://localhost:8080');
        expect(success.context.authMode, AuthMode.legacyToken);
      });

      test('persists token when provided', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);
        await notifier.legacyLogin(
          serverUrl: 'http://localhost:8080',
          token: 'my-secret',
        );

        final contexts = await repo.loadContexts();
        expect(contexts, hasLength(1));
        expect(contexts.single.authToken, 'my-secret');
      });

      test('creates context without token when none provided', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);
        await notifier.legacyLogin(serverUrl: 'http://localhost:8080');

        final contexts = await repo.loadContexts();
        expect(contexts, hasLength(1));
        expect(contexts.single.authToken, isNull);
      });

      test('uses server host as context name', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);
        await notifier.legacyLogin(
          serverUrl: 'http://my-server.example.com:9090',
        );

        final state = container.read(oauthLoginProvider) as OAuthLoginSuccess;
        expect(state.context.name, 'my-server.example.com');
      });

      test('activates context as the active context', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);
        await notifier.legacyLogin(serverUrl: 'http://localhost:8080');

        final activeId = repo.getActiveContextId();
        expect(activeId, isNotNull);

        final state = container.read(oauthLoginProvider) as OAuthLoginSuccess;
        expect(activeId, state.context.id);
      });

      test('upserts existing context with same URL', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);

        // First login.
        await notifier.legacyLogin(
          serverUrl: 'http://localhost:8080',
          token: 'v1',
        );
        final firstId =
            (container.read(oauthLoginProvider) as OAuthLoginSuccess)
                .context
                .id;

        // Second login to same URL.
        await notifier.legacyLogin(
          serverUrl: 'http://localhost:8080',
          token: 'v2',
        );
        final secondId =
            (container.read(oauthLoginProvider) as OAuthLoginSuccess)
                .context
                .id;

        // Same context was updated (same ID), not duplicated.
        expect(firstId, secondId);
        final contexts = await repo.loadContexts();
        expect(contexts, hasLength(1));
        expect(contexts.single.authToken, 'v2');
      });

      test('different URLs create separate contexts', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final notifier = container.read(oauthLoginProvider.notifier);

        await notifier.legacyLogin(serverUrl: 'http://server-a:8080');
        await notifier.legacyLogin(serverUrl: 'http://server-b:9090');

        final contexts = await repo.loadContexts();
        expect(contexts, hasLength(2));
      });
    });

    group('state transitions', () {
      test('legacyLogin emits Loading then Success', () async {
        final repo = await _makeRepo();
        final container = _makeContainer(repo);
        addTearDown(container.dispose);

        final states = <OAuthLoginState>[];
        container.listen(oauthLoginProvider, (prev, next) {
          states.add(next);
        });

        final notifier = container.read(oauthLoginProvider.notifier);
        await notifier.legacyLogin(serverUrl: 'http://localhost:8080');

        // Should have gone through Loading → Success.
        expect(states.length, greaterThanOrEqualTo(2));
        expect(states.first, isA<OAuthLoginLoading>());
        expect((states.first as OAuthLoginLoading).message, 'Connecting...');
        expect(states.last, isA<OAuthLoginSuccess>());
      });
    });
  });
}
