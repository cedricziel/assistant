// Tests that ApiClient.oauth2() wires fresh_dio correctly:
//   1. Proactive refresh when the cached token is expired (no 401 dance).
//   2. Reactive refresh on 401 (the safety net).
//   3. Refresh callback throwing RevokeTokenException → onAuthExpired fires
//      and the request fails with a propagated 401.
//
// All three are covered against a sequenced HttpClientAdapter so we never
// touch the network.

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:fresh_dio/fresh_dio.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/context_token_storage.dart';
import 'package:assistant_app/features/auth/oauth_credentials.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';

class _Reply {
  const _Reply({this.statusCode = 200, this.body = const {}});
  final int statusCode;
  final Map<String, dynamic> body;
}

/// Records each [RequestOptions] and returns a pre-canned sequence of replies
/// per path. Mirrors the helper in auth_interceptor_test.
class _SequencedAdapter implements HttpClientAdapter {
  _SequencedAdapter(this._plan);

  final Map<String, List<_Reply>> _plan;
  final List<RequestOptions> requests = [];

  int countFor(String path) => requests.where((r) => r.path == path).length;

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<dynamic>? cancelFuture,
  ) async {
    requests.add(options);
    final replies = _plan[options.path];
    if (replies == null) {
      throw StateError('No mock plan for ${options.path}');
    }
    final idx = countFor(options.path) - 1;
    if (idx >= replies.length) {
      throw StateError('Plan exhausted for ${options.path} at idx $idx');
    }
    final reply = replies[idx];
    final bytes = utf8.encode(jsonEncode(reply.body));
    return ResponseBody.fromBytes(
      bytes,
      reply.statusCode,
      headers: {
        'content-type': ['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

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
  String accessToken = 'access-current',
  String refreshToken = 'refresh-current',
  required DateTime expiresAt,
}) {
  return OAuthCredentials(
    accessToken: accessToken,
    refreshToken: refreshToken,
    expiresAt: expiresAt,
    clientId: 'client-1',
  );
}

void main() {
  late ContextRepository repo;

  setUp(() async {
    SharedPreferences.setMockInitialValues({});
    final prefs = await SharedPreferences.getInstance();
    repo = ContextRepository(prefs: prefs, secureStorage: _FakeSecureStorage());
  });

  group('ApiClient.oauth2', () {
    test(
      'proactively refreshes an expired token before the first request',
      () async {
        final expired = _creds(
          accessToken: 'expired-jwt',
          expiresAt: DateTime.now().toUtc().subtract(
            const Duration(seconds: 30),
          ),
        );
        final storage = ContextTokenStorage(
          contextId: 'ctx-1',
          repository: repo,
          initial: expired,
        );

        var refreshCalls = 0;
        var expiredCalls = 0;
        final client = ApiClient.oauth2(
          baseUrl: 'http://localhost',
          tokenStorage: storage,
          refreshToken: (old, _) async {
            refreshCalls++;
            return _creds(
              accessToken: 'refreshed-jwt',
              refreshToken: 'refresh-2',
              expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
            );
          },
          onAuthExpired: () async {
            expiredCalls++;
          },
        );

        final adapter = _SequencedAdapter({
          '/api/capabilities': [
            const _Reply(body: {'voice_send': false}),
          ],
        });
        client.dioForTesting.httpClientAdapter = adapter;
        client.refreshDioForTesting?.httpClientAdapter = adapter;

        await client.dioForTesting.get<dynamic>('/api/capabilities');

        expect(
          refreshCalls,
          1,
          reason: 'token was expired → refresh runs before request',
        );
        expect(expiredCalls, 0);
        expect(adapter.countFor('/api/capabilities'), 1);
        expect(
          adapter.requests.single.headers['Authorization'],
          'Bearer refreshed-jwt',
        );
      },
    );

    test('reactive 401 triggers refresh + retry with the new bearer', () async {
      final valid = _creds(
        accessToken: 'looks-valid',
        expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
      );
      final storage = ContextTokenStorage(
        contextId: 'ctx-1',
        repository: repo,
        initial: valid,
      );

      var refreshCalls = 0;
      final client = ApiClient.oauth2(
        baseUrl: 'http://localhost',
        tokenStorage: storage,
        refreshToken: (old, _) async {
          refreshCalls++;
          return _creds(
            accessToken: 'refreshed-after-401',
            refreshToken: 'refresh-2',
            expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
          );
        },
        onAuthExpired: () async {},
      );

      final adapter = _SequencedAdapter({
        '/api/personas': [
          const _Reply(statusCode: 401, body: {'error': 'expired upstream'}),
          const _Reply(body: {'personas': []}),
        ],
      });
      client.dioForTesting.httpClientAdapter = adapter;
      client.refreshDioForTesting?.httpClientAdapter = adapter;

      final response = await client.dioForTesting.get<dynamic>('/api/personas');

      expect(response.statusCode, 200);
      expect(refreshCalls, 1);
      expect(adapter.countFor('/api/personas'), 2);
      expect(
        adapter.requests[1].headers['Authorization'],
        'Bearer refreshed-after-401',
      );
    });

    test(
      'refresh throwing RevokeTokenException fires onAuthExpired and clears credentials',
      () async {
        final valid = _creds(
          accessToken: 'looks-valid',
          expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
        );
        final storage = ContextTokenStorage(
          contextId: 'ctx-1',
          repository: repo,
          initial: valid,
        );

        var expiredCalls = 0;
        final expiredFired = Completer<void>();
        final client = ApiClient.oauth2(
          baseUrl: 'http://localhost',
          tokenStorage: storage,
          refreshToken: (old, _) async {
            throw RevokeTokenException();
          },
          onAuthExpired: () async {
            expiredCalls++;
            if (!expiredFired.isCompleted) expiredFired.complete();
          },
        );

        final adapter = _SequencedAdapter({
          '/api/personas': [
            const _Reply(statusCode: 401, body: {'error': 'expired upstream'}),
          ],
        });
        client.dioForTesting.httpClientAdapter = adapter;
        client.refreshDioForTesting?.httpClientAdapter = adapter;

        Object? caught;
        try {
          await client.dioForTesting.get<dynamic>('/api/personas');
        } on DioException catch (e) {
          caught = e;
        }

        // Wait for the unauthenticated status stream to flush.
        await expiredFired.future.timeout(const Duration(seconds: 1));

        expect(caught, isA<DioException>());
        expect(expiredCalls, greaterThanOrEqualTo(1));
        expect(await storage.read(), isNull, reason: 'fresh_dio cleared cache');
      },
    );
  });
}
