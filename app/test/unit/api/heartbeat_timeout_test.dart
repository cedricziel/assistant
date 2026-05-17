// Locks the contract that `withHeartbeatTimeout` resets its watchdog on ANY
// byte chunk arriving on the underlying stream — including SSE comment bytes
// (`:` + newline). The Axum keep-alive on the server emits `:` comments every
// 15 seconds during byte silence; this client wrapper resets on those bytes
// the same as on `data:` lines.
//
// If a future refactor changes the wrapper to "reset only on parsed events"
// or "reset only on data: lines", this test catches it before the symptom
// (false "stream timed out" errors during slow tool calls) reaches production.
//
// Refs: openspec/changes/sse-keepalive/

import 'dart:async';

import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';

void main() {
  group('withHeartbeatTimeout — keep-alive comment bytes reset the watchdog', () {
    test(
      'comment-only stream at 10s cadence does NOT trip a 60s watchdog over 100s',
      () {
        FakeAsync().run((async) {
          final source = StreamController<List<int>>();
          final wrapped = withHeartbeatTimeout(
            source.stream,
            timeout: const Duration(seconds: 60),
          );

          final chunks = <List<int>>[];
          Object? caughtError;
          final sub = wrapped.listen(
            chunks.add,
            onError: (Object e) => caughtError = e,
          );
          // Let the listener register with the source.
          async.flushMicrotasks();

          // Emit `:\n` every 10 seconds for 100 seconds — 10 chunks total.
          // The 60-second timeout should be reset on each emission.
          for (var i = 0; i < 10; i++) {
            async.elapse(const Duration(seconds: 10));
            source.add(<int>[0x3A, 0x0A]); // ":\n"
            async.flushMicrotasks();
          }
          // Close the source so the watchdog is cancelled before any
          // pending timer can fire. Without this, flushTimers would
          // eventually fire the watchdog at t = lastEmit + 60s.
          source.close();
          async.flushMicrotasks();

          expect(
            caughtError,
            isNull,
            reason:
                'The 60-second watchdog must reset on each 10-second '
                'comment chunk; receiving comments for 100 simulated '
                'seconds must not trip the timeout.',
          );
          expect(
            chunks.length,
            10,
            reason: 'All 10 comment chunks should reach the consumer.',
          );

          sub.cancel();
        });
      },
    );

    test(
      'silence beyond the timeout DOES trip the watchdog (paired negative test)',
      () {
        FakeAsync().run((async) {
          final source = StreamController<List<int>>();
          final wrapped = withHeartbeatTimeout(
            source.stream,
            timeout: const Duration(seconds: 60),
          );

          Object? caughtError;
          final sub = wrapped.listen(
            (_) {},
            onError: (Object e) => caughtError = e,
          );
          async.flushMicrotasks();

          // No bytes arrive. After 70 simulated seconds, the watchdog
          // should have fired at the 60-second mark.
          async.elapse(const Duration(seconds: 70));
          async.flushMicrotasks();

          expect(
            caughtError,
            isA<TimeoutException>(),
            reason:
                '70 seconds of silence with a 60-second timeout must '
                'trigger TimeoutException — the watchdog is the entire '
                'point.',
          );

          sub.cancel();
          source.close();
        });
      },
    );

    test('mixed data-line + comment chunks keep the watchdog reset', () {
      FakeAsync().run((async) {
        final source = StreamController<List<int>>();
        final wrapped = withHeartbeatTimeout(
          source.stream,
          timeout: const Duration(seconds: 30),
        );

        final chunks = <List<int>>[];
        Object? caughtError;
        final sub = wrapped.listen(
          chunks.add,
          onError: (Object e) => caughtError = e,
        );
        async.flushMicrotasks();

        // Pattern: 15s gap, then a chunk, repeated 5 times. Total elapsed
        // 75 seconds with a 30-second watchdog — safe only because every
        // chunk resets it.
        final pattern = <List<int>>[
          <int>[0x3A, 0x0A], // ":\n"
          <int>[0x3A, 0x0A],
          'data: {"event":"token"}\n\n'.codeUnits,
          <int>[0x3A, 0x0A],
          'data: {"event":"done"}\n\n'.codeUnits,
        ];
        for (final chunk in pattern) {
          async.elapse(const Duration(seconds: 15));
          source.add(chunk);
          async.flushMicrotasks();
        }
        source.close();
        async.flushMicrotasks();

        expect(caughtError, isNull);
        expect(chunks.length, 5);

        sub.cancel();
      });
    });
  });
}
