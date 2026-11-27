// Tests that logout removes the browser's push subscription before the
// session is revoked, so the DELETE still carries a valid token.
//
// Spec: openspec/changes/tool-approval-prompts/specs/pwa-push-notifications

import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/auth/oauth_credentials.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/notifications/notification_service.dart';
import 'package:assistant_app/shared/auth_actions.dart';

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

typedef _Seen = ({String method, String path, String? authorization});

void main() {
  test(
    'logout deletes the push subscription while the session is valid',
    () async {
      final seen = <_Seen>[];
      final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      addTearDown(() => server.close(force: true));
      server.listen((request) async {
        seen.add((
          method: request.method,
          path: request.uri.path,
          authorization: request.headers.value('authorization'),
        ));
        try {
          await request.drain<void>();
        } catch (_) {
          // A body we cannot read must not stall the response.
        }
        request.response.statusCode = HttpStatus.ok;
        request.response.headers.contentType = ContentType.json;
        request.response.write('{}');
        await request.response.close();
      });
      final baseUrl = 'http://127.0.0.1:${server.port}';

      SharedPreferences.setMockInitialValues({});
      final prefs = await SharedPreferences.getInstance();
      final repo = ContextRepository(
        prefs: prefs,
        secureStorage: _FakeSecureStorage(),
      );
      final context = AssistantContext(
        id: 'ctx-1',
        name: 'Work',
        serverUrl: baseUrl,
        authMode: AuthMode.oauth2,
        oauthCredentials: OAuthCredentials(
          accessToken: 'valid-access-token',
          refreshToken: 'refresh-token',
          expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
          clientId: 'client-1',
        ),
        createdAt: DateTime.utc(2024, 1, 1),
      );
      await repo.saveContext(context);
      await repo.setActiveContextId(context.id);

      final container = ProviderContainer(
        overrides: [
          contextRepositoryProvider.overrideWithValue(repo),
          // The web implementation has no browser on the VM; the test supplies
          // the endpoint the DELETE should use.
          activeProfileProvider.overrideWithValue(
            ServerProfile(baseUrl: baseUrl, token: 'valid-access-token'),
          ),
        ],
      );
      addTearDown(container.dispose);

      container.read(notificationServiceProvider).debugPushEndpoint =
          'https://push.example/device-1';

      final active = await container.read(activeContextProvider.future);
      expect(active?.oauthCredentials, isNotNull, reason: 'test setup');

      await performLogout(container);

      final deleteIndex = seen.indexWhere(
        (r) => r.method == 'DELETE' && r.path == '/api/push/subscribe',
      );
      expect(
        deleteIndex,
        isNonNegative,
        reason: 'logout must remove the push subscription: $seen',
      );
      expect(
        seen[deleteIndex].authorization,
        'Bearer valid-access-token',
        reason: 'the DELETE must carry the token before it is revoked',
      );

      final revokeIndex = seen.indexWhere(
        (r) => r.method == 'POST' && r.path == '/oauth/revoke',
      );
      expect(
        revokeIndex,
        isNonNegative,
        reason: 'the session is revoked: $seen',
      );
      expect(
        deleteIndex,
        lessThan(revokeIndex),
        reason: 'the subscription is removed while the session is still valid',
      );
    },
  );
}
