import 'dart:async';
import 'dart:io';
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
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
  }) {
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

  /// Number of times [streamEventsFrom] has been called.
  int replayCallCount = 0;

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
    replayCallCount++;
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

  group('ChatMessage.copyWith — audioBytes / audioMimeType', () {
    test('preserves audioBytes and audioMimeType through copyWith', () {
      final bytes = Uint8List.fromList([10, 20, 30]);
      final msg = ChatMessage(
        id: 'v1',
        role: 'user',
        content: 'Hello',
        audioBytes: bytes,
        audioMimeType: 'audio/webm',
      );
      final copy = msg.copyWith(content: 'Updated transcript');
      expect(copy.audioBytes, same(bytes), reason: 'audioBytes preserved');
      expect(
        copy.audioMimeType,
        'audio/webm',
        reason: 'audioMimeType preserved',
      );
      expect(copy.content, 'Updated transcript');
    });

    test('defaults to null when not provided', () {
      final msg = ChatMessage(id: 'v2', role: 'user', content: 'hi');
      expect(msg.audioBytes, isNull);
      expect(msg.audioMimeType, isNull);
    });

    test('copyWith can override audioBytes', () {
      final msg = ChatMessage(
        id: 'v3',
        role: 'user',
        content: 'test',
        audioBytes: Uint8List.fromList([1]),
        audioMimeType: 'audio/mp4',
      );
      final newBytes = Uint8List.fromList([2, 3]);
      final copy = msg.copyWith(audioBytes: newBytes);
      expect(copy.audioBytes, same(newBytes));
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
        ctrl1.addError(const SocketException('network drop'));
        await ctrl1.close();
        // Allow catch block to start.
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      // Pump fake clock to fire the first retry delay (1s backoff).
      await tester.pump(const Duration(seconds: 2));
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

    testWidgets('voice message preserves audioBytes on user ChatMessage', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();
      final audioBytes = Uint8List.fromList([10, 20, 30, 40]);

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendVoiceMessage(audioBytes, 'audio/mp4'));
      await tester.pump();

      final userMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(
        userMsg.audioBytes,
        equals(audioBytes),
        reason: 'user message should carry the recorded audio bytes',
      );
      expect(
        userMsg.audioMimeType,
        equals('audio/mp4'),
        reason: 'user message should carry the audio MIME type',
      );

      ctrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();
    });

    testWidgets('TranscriptEvent updates content but preserves audioBytes', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();
      final audioBytes = Uint8List.fromList([5, 6, 7]);

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendVoiceMessage(audioBytes, 'audio/webm'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(const TranscriptEvent('Hello from voice'));
        await Future<void>.delayed(Duration.zero);
        ctrl
          ..add(const DoneEvent(role: 'assistant', content: 'response'))
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final userMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(
        userMsg.content,
        equals('Hello from voice'),
        reason: 'TranscriptEvent should update user message content',
      );
      expect(
        userMsg.audioBytes,
        equals(audioBytes),
        reason: 'audioBytes must survive the TranscriptEvent copyWith',
      );
      expect(
        userMsg.audioMimeType,
        equals('audio/webm'),
        reason: 'audioMimeType must survive the TranscriptEvent copyWith',
      );
    });
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

  // -- Phase 3: Streaming timeline entry insertion ----------------------------

  group('ChatNotifier streaming — timeline entries', () {
    testWidgets('ThinkingEvent inserts a thinking timeline entry', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('think about it'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(const ThinkingEvent('Let me reason...'));
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();

      final msgs = notifier.state.value!.messages;
      final thinkingEntries = msgs.where(
        (m) => m.timelineType == TimelineEntryType.thinking,
      );
      expect(
        thinkingEntries,
        isNotEmpty,
        reason: 'should have a thinking entry',
      );
      expect(thinkingEntries.first.thinkingContent, 'Let me reason...');

      // Clean up stream.
      ctrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();
    });

    testWidgets('SubagentStartedEvent inserts a subagent timeline entry', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('use subagent'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(
          SubagentStartedEvent.fromJson({
            'agent_id': 'research-1',
            'task': 'Find data',
          }),
        );
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();

      final msgs = notifier.state.value!.messages;
      final subagentEntries = msgs.where(
        (m) => m.timelineType == TimelineEntryType.subagent,
      );
      expect(
        subagentEntries,
        isNotEmpty,
        reason: 'should have a subagent entry',
      );
      expect(subagentEntries.first.subagentId, 'research-1');
      expect(subagentEntries.first.subagentTask, 'Find data');

      ctrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();
    });

    testWidgets(
      'SubagentCompletedEvent updates matching subagent entry summary',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('subagent flow'));
        await tester.pump();

        await tester.runAsync(() async {
          ctrl.add(
            SubagentStartedEvent.fromJson({
              'agent_id': 'r1',
              'task': 'Research',
            }),
          );
          await Future<void>.delayed(Duration.zero);
          ctrl.add(
            SubagentCompletedEvent.fromJson({
              'agent_id': 'r1',
              'status': 'ok',
              'summary': 'Found 3 results',
            }),
          );
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pump();

        final msgs = notifier.state.value!.messages;
        final subagentEntry = msgs.firstWhere(
          (m) =>
              m.timelineType == TimelineEntryType.subagent &&
              m.subagentId == 'r1',
        );
        expect(
          subagentEntry.subagentSummary,
          'Found 3 results',
          reason: 'completed event should update the summary',
        );

        ctrl
          ..add(const DoneEvent(role: 'assistant', content: 'done'))
          ..close();
        await tester.pumpAndSettle();
      },
    );

    testWidgets('multiple ThinkingEvents accumulate content in single entry', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('deep think'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(const ThinkingEvent('First thought. '));
        await Future<void>.delayed(Duration.zero);
        ctrl.add(const ThinkingEvent('Second thought.'));
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();

      final msgs = notifier.state.value!.messages;
      final thinkingEntries = msgs.where(
        (m) => m.timelineType == TimelineEntryType.thinking,
      );
      // Multiple thinking events should accumulate into a single entry.
      expect(thinkingEntries.length, 1);
      expect(
        thinkingEntries.first.thinkingContent,
        'First thought. Second thought.',
      );

      ctrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();
    });
  });

  // -- Phase 3: Timeline entry data model ------------------------------------

  group('ChatMessage timeline entries', () {
    test('default timelineType is message', () {
      final msg = ChatMessage(id: '1', role: 'assistant', content: 'hi');
      expect(msg.timelineType, TimelineEntryType.message);
    });

    test('thinking entry stores content', () {
      final msg = ChatMessage(
        id: '1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.thinking,
        thinkingContent: 'Let me consider...',
      );
      expect(msg.timelineType, TimelineEntryType.thinking);
      expect(msg.thinkingContent, 'Let me consider...');
    });

    test('toolCall entry stores tool data', () {
      final msg = ChatMessage(
        id: '1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(toolName: 'web-search', status: ToolCallStatus.ok),
        ],
      );
      expect(msg.timelineType, TimelineEntryType.toolCall);
      expect(msg.toolCalls.first.toolName, 'web-search');
    });

    test('subagent entry stores agent fields', () {
      final msg = ChatMessage(
        id: '1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.subagent,
        subagentId: 'research-1',
        subagentTask: 'Find weather data',
        subagentSummary: 'Found results',
      );
      expect(msg.timelineType, TimelineEntryType.subagent);
      expect(msg.subagentId, 'research-1');
      expect(msg.subagentTask, 'Find weather data');
      expect(msg.subagentSummary, 'Found results');
    });

    test('copyWith preserves timeline fields', () {
      final msg = ChatMessage(
        id: '1',
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.thinking,
        thinkingContent: 'reasoning',
        subagentId: 'a1',
        subagentTask: 'task',
        subagentSummary: 'summary',
      );
      final copy = msg.copyWith(content: 'updated');
      expect(copy.timelineType, TimelineEntryType.thinking);
      expect(copy.thinkingContent, 'reasoning');
      expect(copy.subagentId, 'a1');
      expect(copy.subagentTask, 'task');
      expect(copy.subagentSummary, 'summary');
    });
  });

  // -- Deferred reconnection on transient errors --------------------------------

  group('ChatNotifier — deferred reconnection', () {
    testWidgets(
      'transient error without run ID shows friendly error immediately',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();

        // Connection closed before RunStartedEvent — no run ID available.
        await tester.runAsync(() async {
          ctrl.addError(
            const HttpException('Connection closed while receiving data'),
          );
          await ctrl.close();
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pumpAndSettle();

        final chatState = notifier.state.value!;
        // No run ID → can't defer → show friendly error immediately.
        expect(chatState.error, isNotNull, reason: 'should have an error set');
        expect(
          chatState.error,
          isNot(contains('HttpException')),
          reason: 'should not expose raw exception type to user',
        );
        expect(
          chatState.error!.toLowerCase(),
          contains('connection lost'),
          reason: 'should show a friendly connection-lost message',
        );
        expect(
          notifier.needsReconnect,
          isFalse,
          reason: 'cannot defer without a run ID',
        );
      },
    );

    testWidgets(
      'transient error with run ID defers reconnection (no error shown)',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();

        await tester.runAsync(() async {
          // Emit run ID so deferred reconnect is possible.
          ctrl.add(const RunStartedEvent('run-defer'));
          await Future<void>.delayed(Duration.zero);
          // Connection closed (iOS backgrounding).
          ctrl.addError(
            const HttpException('Connection closed while receiving data'),
          );
          await ctrl.close();
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        // Pump fake clock through all 3 retry delays (1s + 2s + 4s).
        await tester.pump(const Duration(seconds: 2));
        await tester.pump(const Duration(seconds: 3));
        await tester.pump(const Duration(seconds: 5));
        await tester.pumpAndSettle();

        final chatState = notifier.state.value!;
        expect(
          chatState.error,
          isNull,
          reason: 'transient error with run ID should NOT show error',
        );
        expect(
          notifier.needsReconnect,
          isTrue,
          reason: 'should flag for reconnection on resume',
        );
        expect(
          chatState.isSending,
          isFalse,
          reason: 'should clear sending state',
        );
        // User message should still be in sending status, not failed.
        final userMsg = chatState.messages.firstWhere((m) => m.isUser);
        expect(
          userMsg.status,
          MessageStatus.sending,
          reason:
              'user message stays in sending status until reconnect resolves',
        );
      },
    );

    testWidgets('immediate replay succeeds — no deferred reconnect needed', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      // Replay stream succeeds immediately.
      fakeApi.enqueueReplayStream()
        ..add(const TokenEvent('recovered'))
        ..add(const DoneEvent(role: 'assistant', content: 'recovered'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(const RunStartedEvent('run-replay'));
        await Future<void>.delayed(Duration.zero);
        ctrl.addError(
          const HttpException('Connection closed while receiving data'),
        );
        await ctrl.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      // Pump fake clock for first retry delay (1s backoff).
      await tester.pump(const Duration(seconds: 2));
      await tester.pumpAndSettle();

      final chatState = notifier.state.value!;
      expect(
        chatState.error,
        isNull,
        reason: 'successful replay on first retry should clear the error',
      );
      expect(
        notifier.needsReconnect,
        isFalse,
        reason: 'no deferred reconnect needed when replay works',
      );
      final userMsg = chatState.messages.firstWhere((m) => m.isUser);
      expect(
        userMsg.status,
        MessageStatus.ok,
        reason: 'user message should be ok after successful replay',
      );
    });

    testWidgets('attemptReconnect replays successfully on resume', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      // 3 replay failures for the backoff retries in the catch block.
      for (var i = 0; i < 3; i++) {
        fakeApi.enqueueReplayError(404);
      }
      // Fourth replay (in attemptReconnect) succeeds.
      fakeApi.enqueueReplayStream()
        ..add(const TokenEvent('resumed'))
        ..add(const DoneEvent(role: 'assistant', content: 'resumed'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(const RunStartedEvent('run-resume'));
        await Future<void>.delayed(Duration.zero);
        ctrl.addError(
          const HttpException('Connection closed while receiving data'),
        );
        await ctrl.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      // Pump fake clock through all 3 retry delays.
      await tester.pump(const Duration(seconds: 2));
      await tester.pump(const Duration(seconds: 3));
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();

      expect(notifier.needsReconnect, isTrue);

      // Simulate app resume → attemptReconnect.
      await tester.runAsync(() async {
        await notifier.attemptReconnect();
      });
      await tester.pumpAndSettle();

      final chatState = notifier.state.value!;
      expect(
        chatState.error,
        isNull,
        reason: 'replay on resume should succeed silently',
      );
      expect(
        notifier.needsReconnect,
        isFalse,
        reason: 'reconnect flag should be cleared after attempt',
      );
      final userMsg = chatState.messages.firstWhere((m) => m.isUser);
      expect(
        userMsg.status,
        MessageStatus.ok,
        reason: 'user message should be ok after replay on resume',
      );
    });

    testWidgets(
      'attemptReconnect shows error when both replay and history fail',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();
        // 3 replay failures for the backoff retries in the catch block.
        for (var i = 0; i < 3; i++) {
          fakeApi.enqueueReplayError(404);
        }
        // Fourth replay (in attemptReconnect) also fails.
        fakeApi.enqueueReplayError(404);
        // loadConversation will also fail (fake API has no real server).

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();

        await tester.runAsync(() async {
          ctrl.add(const RunStartedEvent('run-fail'));
          await Future<void>.delayed(Duration.zero);
          ctrl.addError(
            const HttpException('Connection closed while receiving data'),
          );
          await ctrl.close();
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        // Pump fake clock through all 3 retry delays.
        await tester.pump(const Duration(seconds: 2));
        await tester.pump(const Duration(seconds: 3));
        await tester.pump(const Duration(seconds: 5));
        await tester.pumpAndSettle();

        expect(notifier.needsReconnect, isTrue);

        // attemptReconnect: replay fails, loadConversation fails.
        await tester.runAsync(() async {
          await notifier.attemptReconnect();
        });
        await tester.pumpAndSettle();

        final chatState = notifier.state.value!;
        expect(
          chatState.error,
          isNotNull,
          reason: 'should show error when all recovery strategies fail',
        );
        expect(
          chatState.error!.toLowerCase(),
          contains('connection lost'),
          reason: 'should show friendly error message',
        );
        expect(
          notifier.needsReconnect,
          isFalse,
          reason: 'reconnect flag cleared even on failure',
        );
        final userMsg = chatState.messages.firstWhere((m) => m.isUser);
        expect(
          userMsg.status,
          MessageStatus.failed,
          reason: 'user message should be marked failed when all retries fail',
        );
      },
    );

    testWidgets(
      'non-transient error shows immediately (no deferred reconnect)',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();

        await tester.runAsync(() async {
          ctrl.add(const RunStartedEvent('run-nontransient'));
          await Future<void>.delayed(Duration.zero);
          ctrl.addError(const FormatException('malformed response'));
          await ctrl.close();
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pumpAndSettle();

        final chatState = notifier.state.value!;
        expect(
          chatState.error,
          isNotNull,
          reason: 'non-transient errors should show immediately',
        );
        expect(
          notifier.needsReconnect,
          isFalse,
          reason: 'should not defer non-transient errors',
        );
      },
    );

    testWidgets('attemptReconnect is no-op when not needed', (tester) async {
      final fakeApi = _FakeApiClient();

      final notifier = await _pumpTestApp(tester, fakeApi);

      final stateBefore = notifier.state.value;
      await tester.runAsync(() async {
        await notifier.attemptReconnect();
      });
      await tester.pumpAndSettle();

      expect(
        notifier.state.value,
        same(stateBefore),
        reason: 'state should be unchanged when no reconnect is needed',
      );
    });

    testWidgets('clearConversation resets reconnect state', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      // 3 replay failures for the backoff retries in the catch block.
      for (var i = 0; i < 3; i++) {
        fakeApi.enqueueReplayError(404);
      }

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl.add(const RunStartedEvent('run-clear'));
        await Future<void>.delayed(Duration.zero);
        ctrl.addError(
          const HttpException('Connection closed while receiving data'),
        );
        await ctrl.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      // Pump fake clock through all 3 retry delays.
      await tester.pump(const Duration(seconds: 2));
      await tester.pump(const Duration(seconds: 3));
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();

      expect(notifier.needsReconnect, isTrue);

      notifier.clearConversation();

      expect(
        notifier.needsReconnect,
        isFalse,
        reason: 'clearConversation should reset reconnect state',
      );
    });
  });

  // -- Phase: Exponential backoff on stream reconnect -------------------------

  group('ChatNotifier — exponential backoff retries', () {
    testWidgets('retries up to 3 times before deferring to app resume', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      // First stream: emits RunStartedEvent then network error.
      final ctrl1 = fakeApi.enqueueStream();
      // Enqueue 3 replay failures (empty streams that end without DoneEvent).
      for (var i = 0; i < 3; i++) {
        fakeApi.enqueueReplayStream().close();
      }

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      // Deliver RunStartedEvent + error via real async.
      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-backoff'));
        await Future<void>.delayed(Duration.zero);
        ctrl1.addError(const SocketException('Connection reset by peer'));
        await ctrl1.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });

      // The catch block's Future.delayed runs in the fake-async zone.
      // Pump the fake clock forward to fire each retry delay: 1s, 2s, 4s.
      await tester.pump(const Duration(seconds: 2)); // fires 1s delay
      await tester.pump(const Duration(seconds: 3)); // fires 2s delay
      await tester.pump(const Duration(seconds: 5)); // fires 4s delay
      await tester.pumpAndSettle();

      expect(
        fakeApi.replayCallCount,
        equals(3),
        reason: 'should attempt exactly 3 replay retries before giving up',
      );
      expect(
        notifier.needsReconnect,
        isTrue,
        reason: 'after all retries fail, should defer to app resume',
      );
    });

    testWidgets('succeeds on second retry attempt', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      // First replay: empty (fails without DoneEvent → returns false).
      fakeApi.enqueueReplayStream().close();
      // Second replay: success.
      fakeApi.enqueueReplayStream()
        ..add(const TokenEvent('recovered'))
        ..add(const DoneEvent(role: 'assistant', content: 'recovered'))
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('recover'));
      await tester.pump();

      // Deliver RunStartedEvent + error via real async.
      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-recover'));
        await Future<void>.delayed(Duration.zero);
        ctrl1.addError(const SocketException('Connection reset by peer'));
        await ctrl1.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });

      // Pump fake clock: first retry at 1s, second at 2s.
      await tester.pump(const Duration(seconds: 2)); // fires 1s delay
      await tester.pump(const Duration(seconds: 3)); // fires 2s delay
      await tester.pumpAndSettle();

      expect(
        fakeApi.replayCallCount,
        equals(2),
        reason: 'should stop retrying after second attempt succeeds',
      );
      expect(
        notifier.needsReconnect,
        isFalse,
        reason: 'successful retry should not set needsReconnect',
      );
      final userMsg = notifier.state.value!.messages.firstWhere(
        (m) => m.isUser,
      );
      expect(
        userMsg.status,
        MessageStatus.ok,
        reason: 'user message should be ok after successful retry',
      );
    });

    testWidgets('cancelStream during retry backoff stops retrying', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      // Enqueue 3 replays — but we should not reach all of them.
      for (var i = 0; i < 3; i++) {
        fakeApi.enqueueReplayStream().close();
      }

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('cancel-me'));
      await tester.pump();

      // Deliver RunStartedEvent + error via real async.
      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-cancel'));
        await Future<void>.delayed(Duration.zero);
        ctrl1.addError(const SocketException('Connection reset by peer'));
        await ctrl1.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });

      // Let first retry fire (1s delay).
      await tester.pump(const Duration(seconds: 2));
      // Cancel before second retry fires (2s delay).
      notifier.cancelStream();
      // Advance past all remaining retry windows.
      await tester.pump(const Duration(seconds: 10));
      await tester.pumpAndSettle();

      expect(
        fakeApi.replayCallCount,
        lessThanOrEqualTo(1),
        reason: 'cancelStream should prevent further retries',
      );
    });
  });
}
