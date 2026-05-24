// Tests for AuthRecoveryInterceptor — refresh-once-then-deactivate on 401.
//
// Spec: openspec/specs/web-401-recovery/spec.md
//
// Behavior under test:
// - First 401 triggers a single refresh, then retries the request once.
// - Second 401 (already retried) calls onAuthExpired, no further refresh.
// - Refresh failure calls onAuthExpired and propagates the original 401.
// - Non-401 errors pass through unchanged.
// - Concurrent 401s share a single refresh future (single-flight).

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/auth_interceptor.dart';

/// Canned HTTP reply.
class _Reply {
  const _Reply({this.statusCode = 200, this.body = const {}});

  final int statusCode;
  final Map<String, dynamic> body;
}

/// HttpClientAdapter that returns a per-path sequence of replies and records
/// every received [RequestOptions].
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

Dio _makeDio() {
  final dio = Dio(BaseOptions(baseUrl: 'http://localhost'));
  // Match application/json automatically so jsonDecode happens on response.
  dio.options.responseType = ResponseType.json;
  dio.options.headers['Authorization'] = 'Bearer old-token';
  return dio;
}

void main() {
  group('AuthRecoveryInterceptor', () {
    late Dio dio;
    late int refreshCalls;
    late int authExpiredCalls;

    setUp(() {
      dio = _makeDio();
      refreshCalls = 0;
      authExpiredCalls = 0;
    });

    test('first 401 triggers refresh + retry; retry succeeds', () async {
      Future<String?> refresh() async {
        refreshCalls++;
        return 'new-token';
      }

      Future<void> onAuthExpired() async {
        authExpiredCalls++;
      }

      dio.interceptors.add(
        AuthRecoveryInterceptor(
          dio: dio,
          refreshTokens: refresh,
          onAuthExpired: onAuthExpired,
        ),
      );
      final adapter = _SequencedAdapter({
        '/foo': [
          const _Reply(statusCode: 401, body: {'error': 'expired'}),
          const _Reply(statusCode: 200, body: {'ok': true}),
        ],
      });
      dio.httpClientAdapter = adapter;

      final response = await dio.get<dynamic>('/foo');

      expect(response.statusCode, 200);
      expect(response.data, {'ok': true});
      expect(refreshCalls, 1);
      expect(authExpiredCalls, 0);
      expect(adapter.countFor('/foo'), 2);
      // Retry carries the new bearer and the retried marker.
      expect(adapter.requests[1].extra['x-retried'], true);
      expect(adapter.requests[1].headers['Authorization'], 'Bearer new-token');
    });

    test('already-retried 401 calls onAuthExpired and propagates', () async {
      Future<String?> refresh() async {
        refreshCalls++;
        return 'new-token';
      }

      Future<void> onAuthExpired() async {
        authExpiredCalls++;
      }

      dio.interceptors.add(
        AuthRecoveryInterceptor(
          dio: dio,
          refreshTokens: refresh,
          onAuthExpired: onAuthExpired,
        ),
      );
      // Both attempts return 401 — first triggers refresh, retry triggers expire.
      final adapter = _SequencedAdapter({
        '/foo': [
          const _Reply(statusCode: 401, body: {'error': 'expired'}),
          const _Reply(statusCode: 401, body: {'error': 'still expired'}),
        ],
      });
      dio.httpClientAdapter = adapter;

      Object? caught;
      try {
        await dio.get<dynamic>('/foo');
      } on DioException catch (e) {
        caught = e;
      }

      expect(caught, isA<DioException>());
      expect((caught! as DioException).response?.statusCode, 401);
      expect(refreshCalls, 1);
      expect(authExpiredCalls, 1);
      expect(adapter.countFor('/foo'), 2);
    });

    test('refresh returns null → onAuthExpired called, no retry', () async {
      Future<String?> refresh() async {
        refreshCalls++;
        return null; // refresh failed
      }

      Future<void> onAuthExpired() async {
        authExpiredCalls++;
      }

      dio.interceptors.add(
        AuthRecoveryInterceptor(
          dio: dio,
          refreshTokens: refresh,
          onAuthExpired: onAuthExpired,
        ),
      );
      final adapter = _SequencedAdapter({
        '/foo': [
          const _Reply(statusCode: 401, body: {'error': 'expired'}),
        ],
      });
      dio.httpClientAdapter = adapter;

      Object? caught;
      try {
        await dio.get<dynamic>('/foo');
      } on DioException catch (e) {
        caught = e;
      }

      expect(caught, isA<DioException>());
      expect((caught! as DioException).response?.statusCode, 401);
      expect(refreshCalls, 1);
      expect(authExpiredCalls, 1);
      // No retry was attempted.
      expect(adapter.countFor('/foo'), 1);
    });

    test('refresh throws → onAuthExpired called, no retry', () async {
      Future<String?> refresh() async {
        refreshCalls++;
        throw StateError('network down');
      }

      Future<void> onAuthExpired() async {
        authExpiredCalls++;
      }

      dio.interceptors.add(
        AuthRecoveryInterceptor(
          dio: dio,
          refreshTokens: refresh,
          onAuthExpired: onAuthExpired,
        ),
      );
      final adapter = _SequencedAdapter({
        '/foo': [
          const _Reply(statusCode: 401, body: {'error': 'expired'}),
        ],
      });
      dio.httpClientAdapter = adapter;

      Object? caught;
      try {
        await dio.get<dynamic>('/foo');
      } on DioException catch (e) {
        caught = e;
      }

      expect(caught, isA<DioException>());
      expect(refreshCalls, 1);
      expect(authExpiredCalls, 1);
    });

    test('non-401 error passes through without refresh', () async {
      Future<String?> refresh() async {
        refreshCalls++;
        return 'new';
      }

      dio.interceptors.add(
        AuthRecoveryInterceptor(
          dio: dio,
          refreshTokens: refresh,
          onAuthExpired: () async {
            authExpiredCalls++;
          },
        ),
      );
      final adapter = _SequencedAdapter({
        '/foo': [
          const _Reply(statusCode: 500, body: {'error': 'oops'}),
        ],
      });
      dio.httpClientAdapter = adapter;

      Object? caught;
      try {
        await dio.get<dynamic>('/foo');
      } on DioException catch (e) {
        caught = e;
      }

      expect(caught, isA<DioException>());
      expect((caught! as DioException).response?.statusCode, 500);
      expect(refreshCalls, 0);
      expect(authExpiredCalls, 0);
    });

    test(
      'concurrent 401s share a single refresh attempt (single-flight)',
      () async {
        // Refresh holds a gate so two concurrent 401s overlap on it.
        final refreshGate = Completer<void>();
        Future<String?> refresh() async {
          refreshCalls++;
          await refreshGate.future;
          return 'new-token';
        }

        Future<void> onAuthExpired() async {
          authExpiredCalls++;
        }

        dio.interceptors.add(
          AuthRecoveryInterceptor(
            dio: dio,
            refreshTokens: refresh,
            onAuthExpired: onAuthExpired,
          ),
        );
        final adapter = _SequencedAdapter({
          '/a': [
            const _Reply(statusCode: 401, body: {'error': 'a expired'}),
            const _Reply(statusCode: 200, body: {'a': 1}),
          ],
          '/b': [
            const _Reply(statusCode: 401, body: {'error': 'b expired'}),
            const _Reply(statusCode: 200, body: {'b': 1}),
          ],
        });
        dio.httpClientAdapter = adapter;

        final futureA = dio.get<dynamic>('/a');
        final futureB = dio.get<dynamic>('/b');

        // dio defers the actual request dispatch across several event-loop
        // turns, so a single wall-clock delay races *ahead* of the requests
        // and opens the gate before either 401 is even seen — which serialises
        // the two refreshes instead of overlapping them. Drive the loop
        // deterministically instead: pump until both requests have hit their
        // 401 and the single refresh is in flight (parked on the still-closed
        // gate). Because the gate stays closed throughout, the in-flight
        // refresh cannot complete and reset the single-flight slot before the
        // second handler coalesces onto it, so this is order-independent.
        var pumps = 0;
        while (refreshCalls < 1 ||
            adapter.countFor('/a') < 1 ||
            adapter.countFor('/b') < 1) {
          expect(
            pumps++,
            lessThan(100),
            reason: 'requests never reached the refresh path',
          );
          await Future<void>.delayed(Duration.zero);
        }
        // A few more turns so the second 401 handler coalesces onto the
        // in-flight refresh rather than starting its own.
        for (var i = 0; i < 5; i++) {
          await Future<void>.delayed(Duration.zero);
        }
        refreshGate.complete();

        final results = await Future.wait([futureA, futureB]);
        expect(results[0].data, {'a': 1});
        expect(results[1].data, {'b': 1});
        expect(refreshCalls, 1, reason: 'single-flight: only one refresh');
        expect(authExpiredCalls, 0);
      },
    );
  });
}
