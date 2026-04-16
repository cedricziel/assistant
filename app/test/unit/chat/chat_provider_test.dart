import 'dart:async';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fake API client — overrides streamMessages with a controlled stream factory.

class _FakeApiClient extends ApiClient {
  _FakeApiClient() : super(baseUrl: 'http://fake', token: 'fake');

  final List<StreamController<StreamEvent>> _queue = [];
  final List<StreamController<StreamEvent>> _voiceQueue = [];

  /// Enqueue a [StreamController] whose stream will be returned for the next
  /// [streamMessages] call. The caller controls when events are added.
  StreamController<StreamEvent> enqueueStream() {
    final ctrl = StreamController<StreamEvent>();
    _queue.add(ctrl);
    return ctrl;
  }

  /// Enqueue a [StreamController] for the next [sendVoiceMessage] call.
  StreamController<StreamEvent> enqueueVoiceStream() {
    final ctrl = StreamController<StreamEvent>();
    _voiceQueue.add(ctrl);
    return ctrl;
  }

  @override
  Stream<StreamEvent> streamMessages(String conversationId, String message) {
    if (_queue.isNotEmpty) {
      return _queue.removeAt(0).stream;
    }
    return const Stream.empty();
  }

  @override
  Stream<StreamEvent> sendVoiceMessage(
    String conversationId,
    Uint8List audioBytes,
    String mimeType,
  ) {
    if (_voiceQueue.isNotEmpty) {
      return _voiceQueue.removeAt(0).stream;
    }
    return const Stream.empty();
  }

  // Replay queue: each entry is a StreamController<StreamEvent> (success)
  // or a DioException (simulated 404/410).
  final List<dynamic> _replayQueue = [];

  StreamController<StreamEvent> enqueueReplayStream() {
    final ctrl = StreamController<StreamEvent>();
    _replayQueue.add(ctrl);
    return ctrl;
  }

  void enqueueReplayError(int statusCode) {
    _replayQueue.add(
      DioException(
        requestOptions: RequestOptions(path: ''),
        response: Response<void>(
          requestOptions: RequestOptions(path: ''),
          statusCode: statusCode,
        ),
        type: DioExceptionType.badResponse,
      ),
    );
  }

  @override
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
  }) async* {
    if (_replayQueue.isNotEmpty) {
      final item = _replayQueue.removeAt(0);
      if (item is DioException) throw item;
      if (item is StreamController<StreamEvent>) {
        yield* item.stream;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Fake conversation list notifier — no network calls, silent no-ops.

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}

  @override
  void prependConversation(ConversationSummary conv) {}
}

// ---------------------------------------------------------------------------
// Widget scaffold used to drive ProviderScope + get notifier access.

class _TestApp extends ConsumerWidget {
  const _TestApp({required this.onBuild});
  final void Function(ChatNotifier) onBuild;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    onBuild(ref.watch(chatProvider.notifier));
    return const MaterialApp(home: Scaffold());
  }
}

/// Pumps a minimal widget tree with the fake API wired up.
/// Returns the [ChatNotifier] for direct method calls.
Future<ChatNotifier> _pumpTestApp(
  WidgetTester tester,
  _FakeApiClient fakeApi,
) async {
  late ChatNotifier notifier;
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(fakeApi),
        conversationListProvider.overrideWith(
          () => _FakeConversationListNotifier(),
        ),
      ],
      child: _TestApp(onBuild: (n) => notifier = n),
    ),
  );
  await tester.pump();
  notifier.setConversationId('conv-1');
  return notifier;
}

void main() {
  // -- Model tests (pure, no async) -------------------------------------------

  group('MessageStatus enum', () {
    test('has three values: sending, ok, failed', () {
      expect(MessageStatus.values.length, 3);
      expect(MessageStatus.values, contains(MessageStatus.sending));
      expect(MessageStatus.values, contains(MessageStatus.ok));
      expect(MessageStatus.values, contains(MessageStatus.failed));
    });
  });

  group('ChatMessage.copyWith', () {
    test('returns new instance with updated status', () {
      final msg = ChatMessage(id: '1', role: 'user', content: 'hi');
      final updated = msg.copyWith(status: MessageStatus.failed);
      expect(updated.status, MessageStatus.failed);
      expect(msg.status, MessageStatus.ok, reason: 'original unchanged');
    });

    test('preserves original values when no overrides provided', () {
      final msg = ChatMessage(
        id: 'a',
        role: 'user',
        content: 'hello',
        isStreaming: true,
        status: MessageStatus.sending,
      );
      final copy = msg.copyWith();
      expect(copy.id, 'a');
      expect(copy.role, 'user');
      expect(copy.content, 'hello');
      expect(copy.isStreaming, true);
      expect(copy.status, MessageStatus.sending);
    });
  });

  group('ChatState.copyWith with pendingQueue', () {
    test('copies pendingQueue when provided', () {
      const state = ChatState();
      final msgs = [
        const PendingMessage(text: 'a', conversationId: 'c1'),
        const PendingMessage(text: 'b', conversationId: 'c1'),
      ];
      final updated = state.copyWith(pendingQueue: msgs);
      expect(updated.pendingQueue.map((p) => p.text).toList(), ['a', 'b']);
    });

    test('preserves pendingQueue when not overridden', () {
      final state = const ChatState().copyWith(
        pendingQueue: [const PendingMessage(text: 'x', conversationId: 'c1')],
      );
      final again = state.copyWith(isSending: true);
      expect(again.pendingQueue.map((p) => p.text).toList(), ['x']);
    });
  });

  // -- Notifier behaviour tests -----------------------------------------------

  group('ChatNotifier.sendMessage — queue behaviour', () {
    testWidgets('7.1: submitting a second message while first is in flight '
        'adds it to pendingQueue', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream(); // first stream — will hang
      fakeApi
          .enqueueStream() // second stream — completes cleanly
        ..add(const DoneEvent(role: 'assistant', content: 'r2'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      // First message — starts drain, which opens the hanging stream.
      unawaited(notifier.sendMessage('first'));
      await tester.pump();

      // Second message — drain is already running, so it must queue.
      unawaited(notifier.sendMessage('second'));
      await tester.pump();

      expect(
        notifier.state.value!.pendingQueue.map((p) => p.text),
        contains('second'),
        reason: 'second message should be in pendingQueue while first streams',
      );

      // Complete first stream to allow drain to finish cleanly.
      ctrl1
        ..add(const DoneEvent(role: 'assistant', content: 'r1'))
        ..close();
      await tester.pumpAndSettle();
      // ctrl2 was pre-completed above; drain will process it and finish.
    });

    testWidgets('7.2: queue drains in FIFO order after DoneEvent', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      final ctrl2 = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('first'));
      await tester.pump();
      unawaited(notifier.sendMessage('second'));
      await tester.pump();

      // Stream events are only delivered via real async scheduling (not fake-async
      // frames), so we use runAsync to drive both completions in order.
      await tester.runAsync(() async {
        ctrl1
          ..add(const DoneEvent(role: 'assistant', content: 'r1'))
          ..close();
        // Yield to the event loop so _drainQueue can resume and
        // _streamMessage('second') can subscribe to ctrl2 before we add events.
        await Future<void>.delayed(Duration.zero);
        ctrl2
          ..add(const DoneEvent(role: 'assistant', content: 'r2'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      // Both user messages should appear in submission order.
      final userMessages = notifier.state.value!.messages
          .where((m) => m.isUser)
          .map((m) => m.content)
          .toList();
      expect(
        userMessages,
        orderedEquals(['first', 'second']),
        reason: 'messages must appear in submission order (FIFO)',
      );
    });

    testWidgets(
      '7.3: ErrorEvent leaves user message in list with status == failed',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('oops'));
        await tester.pump();

        ctrl
          ..add(const ErrorEvent('server error'))
          ..close();
        await tester.pumpAndSettle();

        final chatState = notifier.state.value!;
        final userMsg = chatState.messages.firstWhere((m) => m.isUser);
        expect(
          userMsg.status,
          MessageStatus.failed,
          reason: 'user message must be marked failed after ErrorEvent',
        );
        expect(
          chatState.messages.any((m) => m.id == 'assistant-streaming'),
          isFalse,
          reason: 'streaming placeholder must be removed after error',
        );
      },
    );

    testWidgets('7.5: AudioReadyEvent sets audioId on the assistant message', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('play audio'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl
          ..add(const AudioReadyEvent('uuid-123'))
          ..add(const DoneEvent(role: 'assistant', content: 'Here you go'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final assistant = notifier.state.value!.messages.firstWhere(
        (m) => !m.isUser,
      );
      expect(
        assistant.audioId,
        equals('uuid-123'),
        reason: 'AudioReadyEvent should set audioId on the assistant message',
      );
    });

    testWidgets('7.4: retryMessage removes failed message and re-enqueues it', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream(); // first attempt → error
      fakeApi
          .enqueueStream() // retry → success
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('retry-me'));
      await tester.pump();

      // First attempt fails — use runAsync so the _drainQueue finally block
      // (level-2 async) runs and _draining is reset to false before retry.
      await tester.runAsync(() async {
        ctrl1
          ..add(const ErrorEvent('timeout'))
          ..close();
        await Future<void>.delayed(Duration.zero); // deliver ErrorEvent
        await Future<void>.delayed(Duration.zero); // run _drainQueue finally
      });
      await tester.pumpAndSettle();

      final failedMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(failedMsg.status, MessageStatus.failed);

      // Retry — ctrl2 is pre-buffered; use runAsync to let it deliver after
      // _drainQueue restarts.
      unawaited(notifier.retryMessage(failedMsg));
      await tester.runAsync(() async {
        await Future<void>.delayed(
          Duration.zero,
        ); // _drainQueue starts _streamMessage
        await Future<void>.delayed(Duration.zero); // ctrl2 DoneEvent delivered
      });
      await tester.pumpAndSettle();

      // Failed message must be removed.
      expect(
        notifier.state.value!.messages.any((m) => m.id == failedMsg.id),
        isFalse,
        reason: 'failed message must be removed from list on retry',
      );

      // Retried message must be visible (currently streaming or ok after ctrl2 fires).
      expect(
        notifier.state.value!.messages.any(
          (m) => m.isUser && m.content == 'retry-me',
        ),
        isTrue,
        reason: 'retried message must reappear in list during drain',
      );
    });
  });

  // -- reconnect via event-log replay -----------------------------------------

  group('ChatNotifier event-log replay', () {
    testWidgets('8.1: successful reconnect replays tokens into UI', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      // First stream: emits RunStartedEvent then throws (network drop).
      final ctrl1 = fakeApi.enqueueStream();
      // Replay stream: pre-populated with tokens and done.
      fakeApi.enqueueReplayStream()
        ..add(const TokenEvent('hello'))
        ..add(const DoneEvent(role: 'assistant', content: 'hello'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('ping'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-abc'));
        await Future<void>.delayed(Duration.zero); // deliver RunStartedEvent
        ctrl1.addError(Exception('network drop'));
        await ctrl1.close();
        // Allow catch block + _replayRun to run.
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final chatState = notifier.state.value!;
      final userMsg = chatState.messages.firstWhere((m) => m.isUser);
      expect(
        userMsg.status,
        MessageStatus.ok,
        reason: 'user message should be ok after successful replay',
      );
      final assistant = chatState.messages.firstWhere((m) => !m.isUser);
      expect(
        assistant.content,
        'hello',
        reason: 'replayed tokens should appear as assistant message',
      );
    });

    testWidgets('8.2: replay 404 falls back to re-send', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      fakeApi.enqueueReplayError(404);
      // Re-send stream: success.
      fakeApi.enqueueStream()
        ..add(const DoneEvent(role: 'assistant', content: 'resent'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hi'));
      await tester.pump();

      // First stream: capture run ID, then ErrorEvent → failed.
      await tester.runAsync(() async {
        ctrl1
          ..add(const RunStartedEvent('run-xyz'))
          ..add(const ErrorEvent('server error'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final failedMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(failedMsg.status, MessageStatus.failed);

      // Retry: _replayRun → 404 → fallback → sendMessage → ctrl3 success.
      unawaited(notifier.retryMessage(failedMsg));
      await tester.runAsync(() async {
        await Future<void>.delayed(Duration.zero); // _replayRun called
        await Future<void>.delayed(Duration.zero); // 404 thrown + handled
        await Future<void>.delayed(
          Duration.zero,
        ); // sendMessage queued + drained
        await Future<void>.delayed(Duration.zero); // ctrl3 DoneEvent delivered
      });
      await tester.pumpAndSettle();

      expect(
        notifier.state.value!.messages.any((m) => m.id == failedMsg.id),
        isFalse,
        reason: 'original failed message should be removed on fallback retry',
      );
      expect(
        notifier.state.value!.messages.any(
          (m) => m.isUser && m.content == 'hi',
        ),
        isTrue,
        reason: 're-sent message should reappear',
      );
    });

    testWidgets('8.3: replay 410 falls back to re-send', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      fakeApi.enqueueReplayError(410);
      fakeApi.enqueueStream()
        ..add(const DoneEvent(role: 'assistant', content: 'ok'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('test'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl1
          ..add(const RunStartedEvent('run-410'))
          ..add(const ErrorEvent('stream gone'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final failedMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(failedMsg.status, MessageStatus.failed);

      unawaited(notifier.retryMessage(failedMsg));
      await tester.runAsync(() async {
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      expect(
        notifier.state.value!.messages.any((m) => m.id == failedMsg.id),
        isFalse,
        reason: '410 replay should fall back and remove original failed msg',
      );
      expect(
        notifier.state.value!.messages.any(
          (m) => m.isUser && m.content == 'test',
        ),
        isTrue,
      );
    });
  });

  // -- sendVoiceMessage tests -------------------------------------------------

  group('ChatNotifier.sendVoiceMessage', () {
    testWidgets('adds a voice placeholder user message while streaming', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(
        notifier.sendVoiceMessage(Uint8List.fromList([1, 2, 3]), 'audio/webm'),
      );
      await tester.pump();

      final chatState = notifier.state.value!;
      expect(chatState.isSending, isTrue);
      final userMsg = chatState.messages.firstWhere((m) => m.isUser);
      expect(userMsg.content, contains('🎤'));
      expect(userMsg.status, MessageStatus.sending);

      ctrl
        ..add(const DoneEvent(role: 'assistant', content: 'transcribed'))
        ..close();
      await tester.pumpAndSettle();
    });

    testWidgets('DoneEvent finalizes the assistant message', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(
        notifier.sendVoiceMessage(Uint8List.fromList([1, 2, 3]), 'audio/webm'),
      );
      await tester.pump();

      await tester.runAsync(() async {
        ctrl
          ..add(const DoneEvent(role: 'assistant', content: 'Hello!'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final assistant = notifier.state.value!.messages.firstWhere(
        (m) => !m.isUser,
      );
      expect(assistant.content, equals('Hello!'));
      expect(assistant.isStreaming, isFalse);
      expect(notifier.state.value!.isSending, isFalse);
    });

    testWidgets('ErrorEvent marks user message as failed', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(
        notifier.sendVoiceMessage(Uint8List.fromList([1, 2, 3]), 'audio/webm'),
      );
      await tester.pump();

      ctrl
        ..add(const ErrorEvent('transcription failed'))
        ..close();
      await tester.pumpAndSettle();

      final userMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(userMsg.status, MessageStatus.failed);
    });

    testWidgets(
      'AudioReadyEvent sets audioId on the streaming assistant message',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueVoiceStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(
          notifier.sendVoiceMessage(
            Uint8List.fromList([1, 2, 3]),
            'audio/webm',
          ),
        );
        await tester.pump();

        await tester.runAsync(() async {
          ctrl
            ..add(const AudioReadyEvent('voice-audio-id'))
            ..add(const DoneEvent(role: 'assistant', content: 'Here'))
            ..close();
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pumpAndSettle();

        final assistant = notifier.state.value!.messages.firstWhere(
          (m) => !m.isUser,
        );
        expect(assistant.audioId, equals('voice-audio-id'));
      },
    );
  });

  // -- tokenStream tests ------------------------------------------------------

  group('ChatNotifier.tokenStream', () {
    testWidgets('streaming assistant message has a tokenStream', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      // Grab the token stream from the streaming assistant message.
      final streamingMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.id == 'assistant-streaming',
      );
      expect(
        streamingMsg.tokenStream,
        isNotNull,
        reason: 'streaming message should expose a tokenStream',
      );

      // Complete the stream cleanly.
      await tester.runAsync(() async {
        ctrl
          ..add(const DoneEvent(role: 'assistant', content: 'done'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      // After DoneEvent, the final message should not have a tokenStream.
      final finalMsg = notifier.state.value!.messages.firstWhere(
        (m) => !m.isUser,
      );
      expect(
        finalMsg.tokenStream,
        isNull,
        reason: 'finalized message should not carry a tokenStream',
      );
    });
  });
}
