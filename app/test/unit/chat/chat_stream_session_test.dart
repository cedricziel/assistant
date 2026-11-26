import 'dart:async';

import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_stream_session.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('ChatStreamSession.beginTurn', () {
    test('resets detach, run id, cursor and deferred reconnect', () {
      final session = ChatStreamSession()
        ..detach()
        ..runId = 'run-1'
        ..lastSeq = 9
        ..markDisconnected(userMsgId: 'u1', conversationId: 'c1');

      session.beginTurn();

      expect(session.detached, isFalse);
      expect(session.runId, isNull);
      expect(session.lastSeq, 0);
      expect(session.needsReconnect, isFalse);
      expect(session.takeDisconnected(), isNull);
    });

    test('keeps the queue-drain flag', () {
      final session = ChatStreamSession()..draining = true;
      session.beginTurn();
      expect(session.draining, isTrue);
    });
  });

  group('ChatStreamSession.untilDetached', () {
    test('closes the stream on detach without another source event', () async {
      final session = ChatStreamSession();
      final source = StreamController<int>();
      final received = <int>[];
      final done = Completer<void>();
      session
          .untilDetached(source.stream)
          .listen(received.add, onDone: done.complete);

      source.add(1);
      await Future<void>.delayed(Duration.zero);
      session.detach();
      await done.future.timeout(const Duration(seconds: 1));

      expect(received, [1]);
      expect(source.hasListener, isFalse);
      expect(session.detached, isTrue);
    });

    test('passes events and completion through when never detached', () async {
      final session = ChatStreamSession();
      final out = await session
          .untilDetached(Stream.fromIterable([1, 2, 3]))
          .toList();
      expect(out, [1, 2, 3]);
    });

    test('a new turn gets a fresh detach', () {
      final session = ChatStreamSession()..detach();
      session.beginTurn();
      expect(session.detached, isFalse);
    });
  });

  group('ChatStreamSession.advance', () {
    test('moves the cursor one past the event sequence id', () {
      final session = ChatStreamSession()
        ..advance(const TokenEvent('a', sequenceId: 4));
      expect(session.lastSeq, 5);
    });

    test('moves one past the current cursor when the event has no id', () {
      final session = ChatStreamSession()
        ..lastSeq = 3
        ..advance(const TokenEvent('a'));
      expect(session.lastSeq, 4);
    });
  });

  group('ChatStreamSession deferred reconnect', () {
    test('markDisconnected arms needsReconnect', () {
      final session = ChatStreamSession()
        ..markDisconnected(userMsgId: 'u1', conversationId: 'c1');
      expect(session.needsReconnect, isTrue);
    });

    test(
      'takeDisconnected returns the interrupted turn once and clears it',
      () {
        final session = ChatStreamSession()
          ..markDisconnected(userMsgId: 'u1', conversationId: 'c1');

        final taken = session.takeDisconnected();

        expect(taken?.userMsgId, 'u1');
        expect(taken?.conversationId, 'c1');
        expect(session.needsReconnect, isFalse);
        expect(session.takeDisconnected(), isNull);
      },
    );

    test('takeDisconnected keeps the run id and cursor for the replay', () {
      final session = ChatStreamSession()
        ..runId = 'run-1'
        ..lastSeq = 7
        ..markDisconnected(userMsgId: 'u1', conversationId: 'c1');
      session.takeDisconnected();
      expect(session.runId, 'run-1');
      expect(session.lastSeq, 7);
    });

    test('clearDisconnected drops the deferred turn', () {
      final session = ChatStreamSession()
        ..markDisconnected(userMsgId: 'u1', conversationId: 'c1')
        ..clearDisconnected();
      expect(session.needsReconnect, isFalse);
      expect(session.takeDisconnected(), isNull);
    });
  });

  group('ChatStreamSession thinking stream', () {
    test(
      'openThinking exposes a broadcast stream and replaces any open one',
      () async {
        final session = ChatStreamSession();
        expect(session.thinking, isNull);

        final first = session.openThinking();
        expect(first.stream.isBroadcast, isTrue);
        final second = session.openThinking();
        expect(identical(session.thinking, second), isTrue);
        expect(identical(first, second), isFalse);
      },
    );

    test('closeThinking closes the stream and clears it', () async {
      final session = ChatStreamSession();
      final controller = session.openThinking();
      var done = false;
      controller.stream.listen((_) {}, onDone: () => done = true);

      session.closeThinking();
      await Future<void>.delayed(Duration.zero);

      expect(done, isTrue);
      expect(session.thinking, isNull);
    });

    test('closeThinking is a no-op when nothing is open', () {
      ChatStreamSession().closeThinking();
    });
  });

  group('ChatStreamSession.retryDelay', () {
    test('doubles from one second per attempt', () {
      expect(ChatStreamSession.retryDelay(0), const Duration(seconds: 1));
      expect(ChatStreamSession.retryDelay(1), const Duration(seconds: 2));
      expect(ChatStreamSession.retryDelay(2), const Duration(seconds: 4));
    });
  });
}
