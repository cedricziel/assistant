import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_announcer.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_stream_session.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_ui/assistant_ui.dart' show kTurnStallThreshold;
import 'package:built_collection/built_collection.dart';
import 'package:built_value/json_object.dart';
import 'package:dio/dio.dart';
import 'package:material_ui/material_ui.dart';
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

  /// Messages passed to [streamMessages], in call order.
  final List<String> sentMessages = [];

  @override
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
  }) {
    sentMessages.add(message);
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

  /// Texts passed to [streamEventsFrom], in call order.
  final List<({String conversationId, String runId, int since})> replayCalls =
      [];

  @override
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
    CancelToken? cancelToken,
  }) async* {
    replayCallCount++;
    replayCalls.add((
      conversationId: conversationId,
      runId: runId,
      since: since,
    ));
    if (_replayQueue.isNotEmpty) {
      final item = _replayQueue.removeAt(0);
      if (item is DioException) throw item;
      if (item is StreamController<StreamEvent>) {
        yield* item.stream;
      }
    }
  }

  // -- Turn-status probe stub ------------------------------------------------
  //
  // Configure [nextTurnStatus] to drive `ChatNotifier`'s initial-stall
  // watchdog. `null` (default) means "probe failed" → notifier falls back
  // to its recovery path.

  /// Value returned by the next [turnStatus] call. `null` means the probe
  /// throws or the server returns no body — both paths fall back to the
  /// legacy recovery in the notifier.
  TurnState? nextTurnStatus;

  /// Captures each `(conversationId, runId)` the notifier probes for.
  final List<({String conversationId, String runId})> turnStatusCalls = [];

  @override
  Future<TurnState?> turnStatus(String conversationId, String runId) async {
    turnStatusCalls.add((conversationId: conversationId, runId: runId));
    return nextTurnStatus;
  }

  /// Captures each `(conversationId, runId)` the notifier asks to cancel.
  final List<({String conversationId, String runId})> cancelTurnCalls = [];

  @override
  Future<TurnState?> cancelTurn(String conversationId, String turnId) async {
    cancelTurnCalls.add((conversationId: conversationId, runId: turnId));
    if (cancelResults.isEmpty) return TurnState.running;
    return cancelResults.removeAt(0);
  }

  /// Results for successive [cancelTurn] calls (`null` = request failed);
  /// `running` once exhausted.
  final List<TurnState?> cancelResults = [];

  final _FakeConversationsApi _conversationsApi = _FakeConversationsApi();

  /// The conversation `GET` the notifier refreshes from.
  @override
  ConversationsApi get conversations => _conversationsApi;

  /// Gates and scripts the next `getConversation` reply.
  _FakeConversationsApi get conversationsFake => _conversationsApi;

  /// Runs reported by the pending-interactions list.
  List<InteractionDetail> pendingInteractions = [];

  /// Every status filter the pending list was asked for.
  final List<InteractionStatusView?> interactionStatusQueries = [];

  int pendingListCalls = 0;

  @override
  Future<List<InteractionDetail>> listConversationInteractions(
    String conversationId, {
    InteractionStatusView? status,
  }) async {
    pendingListCalls++;
    interactionStatusQueries.add(status);
    return pendingInteractions;
  }
}

class _FakeConversationsApi extends ConversationsApi {
  _FakeConversationsApi() : super(Dio(), standardSerializers);

  /// Messages the server reports, as `(id, role, content)`. While `null`
  /// (the default) the request fails, like an unreachable server.
  List<(String, String, String)>? serverMessages;

  /// Interactions the conversation detail reports, resolved and pending.
  List<InteractionDetail> serverInteractions = [];

  /// When set, used verbatim instead of [serverMessages].
  List<MessageSummary>? serverMessageSummaries;

  /// `finish_reason` per message id; ids not listed have none.
  final Map<String, String> finishReasons = {};

  /// When set, `getConversation` waits for it before answering.
  Completer<void>? gate;

  int getConversationCalls = 0;

  @override
  Future<Response<ConversationDetail>> getConversation({
    required String id,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    getConversationCalls++;
    await gate?.future;
    final messages = serverMessages;
    if (messages == null && serverMessageSummaries == null) {
      throw DioException(
        requestOptions: RequestOptions(path: '/api/conversations/$id'),
        type: DioExceptionType.connectionError,
      );
    }
    final rows = messages ?? const <(String, String, String)>[];
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id'),
      statusCode: 200,
      data: ConversationDetail(
        (b) => b
          ..id = id
          ..title = 'T'
          ..createdAt = DateTime(2024, 1, 1)
          ..updatedAt = DateTime(2024, 1, 1)
          ..interactions.replace(serverInteractions)
          ..messages.replace(
            serverMessageSummaries ??
                [
                  for (final (mid, role, content) in rows)
                    MessageSummary(
                      (m) => m
                        ..id = mid
                        ..role = role
                        ..content = content
                        ..createdAt = DateTime(2024, 1, 1)
                        ..turn = 0
                        ..ttsAvailable = false
                        ..finishReason = finishReasons[mid],
                    ),
                ],
          ),
      ),
    );
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
    test('has four values: sending, ok, failed, stopped', () {
      expect(MessageStatus.values.length, 4);
      expect(MessageStatus.values, contains(MessageStatus.stopped));
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

  // -- response timing --------------------------------------------------------

  group('ChatNotifier — response timing', () {
    const timing = MessageTiming(
      timeToFirstToken: Duration(milliseconds: 800),
      total: Duration(milliseconds: 3000),
      outputTokens: 120,
      decode: Duration(milliseconds: 2200),
    );

    testWidgets('a live done frame attaches its timing to the reply', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl
          ..add(
            const DoneEvent(
              role: 'assistant',
              content: 'Hi',
              messageId: 'a1',
              timing: timing,
            ),
          )
          ..close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final assistant = notifier.state.value!.messages.firstWhere(
        (m) => !m.isUser,
      );
      expect(assistant.timing?.total, const Duration(milliseconds: 3000));
      expect(assistant.timing?.outputTokens, 120);
    });

    testWidgets('a replayed done frame attaches the same timing', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      fakeApi.enqueueReplayStream()
        ..add(const TokenEvent('Hi'))
        ..add(
          const DoneEvent(
            role: 'assistant',
            content: 'Hi',
            messageId: 'a1',
            timing: timing,
          ),
        )
        ..close();

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('ping'));
      await tester.pump();

      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-abc'));
        await Future<void>.delayed(Duration.zero);
        ctrl1.addError(const SocketException('network drop'));
        await ctrl1.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump(const Duration(seconds: 2));
      await tester.pumpAndSettle();

      final assistant = notifier.state.value!.messages.firstWhere(
        (m) => !m.isUser,
      );
      expect(
        assistant.timing?.total,
        const Duration(milliseconds: 3000),
        reason: 'replay through the event log keeps the same timing',
      );
      expect(
        assistant.timing?.timeToFirstToken,
        const Duration(milliseconds: 800),
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

    testWidgets(
      'stopTurn during retry backoff cancels on the server and stops retrying',
      (tester) async {
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
        notifier.stopTurn();
        // Advance past all remaining retry windows.
        await tester.pump(const Duration(seconds: 10));
        await tester.pumpAndSettle();

        expect(
          fakeApi.replayCallCount,
          lessThanOrEqualTo(1),
          reason: 'stopTurn should prevent further retries',
        );
        expect(fakeApi.cancelTurnCalls.map((c) => c.runId), ['run-cancel']);
        final s = notifier.state.value!;
        expect(s.isSending, isFalse);
        expect(s.isStopping, isFalse);
      },
    );
  });

  // -- Stalled-stream recovery (iPhone/iPad bug: dots forever) ---------------
  //
  // Reproduces the user-reported behaviour:
  //   "I send a message and I see the three jumpy dots indefinitely on my
  //    iPhone. […] reload the conversation via menu and a new message
  //    appears."
  //
  // What we know from that report:
  //   - POST /messages succeeds: the server receives, runs, and persists
  //     the assistant reply (it's there on reload).
  //   - The SSE response stream never delivers any events to the iPhone
  //     in real time (likely iOS Dio / dart:io HttpClient buffering).
  //   - ChatNotifier's existing recovery is the byte-level heartbeat
  //     timeout (90 s) followed by up to three replay-run retries, each
  //     with its own 90 s budget. That is "forever" from a user POV.
  //
  // This test pins a stronger user contract: a stream that delivers no
  // useful events within ~15 s of the user pressing Send MUST NOT leave
  // the UI in the "isStreaming, no content, dots showing" state. The fix
  // is intentionally not prescribed by this test — surfacing an error,
  // polling the conversation endpoint, exposing a retry button, or
  // shortening the no-progress deadline are all acceptable.
  group('ChatNotifier — stalled SSE stream recovery', () {
    testWidgets('stream that emits no tokens within 15s does not leave the UI in '
        'indefinite "streaming dots" state', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });

      final notifier = await _pumpTestApp(tester, fakeApi);

      // User hits Send. The notifier inserts a user bubble and an
      // empty assistant-streaming placeholder, then awaits the SSE
      // stream.
      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      // Server acknowledges the run (matches the X-Run-Id header path)
      // but never emits a token, status, or done event afterwards —
      // exactly what iOS sees when Dio buffers the chunked body.
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();

      // Precondition: we are in the "dots" state right now.
      final initial = notifier.state.value!;
      final initialPlaceholder = initial.messages.firstWhere(
        (m) => m.id == 'assistant-streaming',
        orElse: () =>
            ChatMessage(id: 'missing', role: 'assistant', content: ''),
      );
      expect(
        initialPlaceholder.isStreaming && initialPlaceholder.content.isEmpty,
        isTrue,
        reason:
            'precondition: right after Send we expect the dots-state '
            'placeholder to be present',
      );

      // Advance time past the no-progress watchdog. 15 s is well above
      // what a user will tolerate while staring at jumpy dots, and well
      // below the current 90 s heartbeat × 3-retry total. The trailing
      // pump drains microtasks scheduled by the timeout-driven error
      // propagation through the stream subscription.
      await tester.pump(const Duration(seconds: 15));
      await tester.pumpAndSettle();

      final after = notifier.state.value!;
      final stillInDots = after.messages.any(
        (m) =>
            m.id == 'assistant-streaming' && m.isStreaming && m.content.isEmpty,
      );

      expect(
        stillInDots,
        isFalse,
        reason:
            'After 15s with no SSE progress the UI must exit the '
            '"dots forever" state — surface an error, fall back to a '
            'fetch of the conversation, or clear the placeholder. '
            'Current state: isSending=${after.isSending}, '
            'error=${after.error}, '
            'messages=${after.messages.map((m) => '(${m.id}, streaming=${m.isStreaming}, content="${m.content}")').toList()}',
      );
    });

    // The voice-send path has the same shape as the text path: it inserts a
    // voice placeholder + an empty assistant-streaming bubble and then awaits
    // an SSE stream. Same iOS Dio buffering risk → same user contract: a
    // stream that delivers no useful events within ~15 s must not leave the
    // UI in dots state.
    testWidgets('voice stream that emits no events within 15s does not leave the '
        'UI in indefinite "streaming dots" state', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(
        notifier.sendVoiceMessage(Uint8List.fromList([1, 2, 3]), 'audio/webm'),
      );
      await tester.pump();

      // Precondition: the assistant-streaming placeholder is showing dots.
      final initial = notifier.state.value!;
      final initialPlaceholder = initial.messages.firstWhere(
        (m) => m.id == 'assistant-streaming',
        orElse: () =>
            ChatMessage(id: 'missing', role: 'assistant', content: ''),
      );
      expect(
        initialPlaceholder.isStreaming && initialPlaceholder.content.isEmpty,
        isTrue,
        reason:
            'precondition: voice send should insert the dots-state '
            'placeholder',
      );

      await tester.pump(const Duration(seconds: 15));
      await tester.pumpAndSettle();

      final after = notifier.state.value!;
      final stillInDots = after.messages.any(
        (m) =>
            m.id == 'assistant-streaming' && m.isStreaming && m.content.isEmpty,
      );
      expect(
        stillInDots,
        isFalse,
        reason:
            'After 15s with no voice-stream progress the UI must exit the '
            '"dots forever" state. Current: isSending=${after.isSending}, '
            'error=${after.error}, '
            'messages=${after.messages.map((m) => '(${m.id}, streaming=${m.isStreaming}, content="${m.content}")').toList()}',
      );
    });
  });

  // -- Turn-status probe routes the initial-stall watchdog ------------------
  //
  // When `_initialStallTimeout` fires before any UI-visible event, the
  // notifier now consults `api.turnStatus` instead of unconditionally
  // recovering. The decision matrix:
  //
  //   server says `running`       → keep the sink open (don't clear dots)
  //   server says `completed`     → recover (clear placeholder, refetch)
  //   server says `errored`       → recover (clear placeholder, refetch)
  //   server says `unknown`       → recover (server doesn't recognise turn)
  //   probe fails / returns null  → recover (fallback to legacy behaviour)

  group('ChatNotifier — stall probe routing', () {
    testWidgets('probe=running keeps the placeholder in the dots state', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      fakeApi.nextTurnStatus = TurnState.running;
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();

      // RunStarted gives the notifier a run id to probe with — without it
      // the probe path is bypassed and the legacy recovery runs instead.
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();

      await tester.pump(const Duration(seconds: 15));
      await tester.pumpAndSettle();

      final after = notifier.state.value!;
      final placeholder = after.messages.firstWhere(
        (m) => m.id == 'assistant-streaming',
        orElse: () =>
            ChatMessage(id: 'missing', role: 'assistant', content: ''),
      );
      expect(
        placeholder.id == 'assistant-streaming' &&
            placeholder.isStreaming &&
            placeholder.content.isEmpty,
        isTrue,
        reason:
            'probe=running must leave the streaming placeholder in place '
            'so the 90 s byte-watchdog (not the 12 s app watchdog) decides '
            'whether to recover. Current: '
            'isSending=${after.isSending}, error=${after.error}, '
            'messages=${after.messages.map((m) => '(${m.id}, streaming=${m.isStreaming})').toList()}',
      );
      expect(
        after.error,
        isNull,
        reason: 'no error surfaced on a healthy turn',
      );
      expect(after.isSending, isTrue, reason: 'still sending');
      expect(
        fakeApi.turnStatusCalls,
        isNotEmpty,
        reason: 'probe must have been invoked',
      );
      expect(fakeApi.turnStatusCalls.first.runId, 'run-1');

      // Drain the controller before teardown so the await-for completes.
      ctrl.add(const DoneEvent(role: 'assistant', content: ''));
      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('probe=awaitingInput keeps the placeholder like running', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      fakeApi.nextTurnStatus = TurnState.awaitingInput;
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();

      await tester.pump(const Duration(seconds: 15));
      await tester.pumpAndSettle();

      final after = notifier.state.value!;
      expect(fakeApi.turnStatusCalls, isNotEmpty, reason: 'probe must run');
      expect(
        after.messages.any(
          (m) =>
              m.id == 'assistant-streaming' &&
              m.isStreaming &&
              m.content.isEmpty,
        ),
        isTrue,
        reason:
            'a turn waiting for the owner is alive: recovery would drop the '
            'placeholder and lose the pending approval',
      );
      expect(after.error, isNull);
      expect(after.isSending, isTrue);

      ctrl.add(const DoneEvent(role: 'assistant', content: ''));
      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets(
      'probe=completed triggers recovery and clears the placeholder',
      (tester) async {
        final fakeApi = _FakeApiClient();
        fakeApi.nextTurnStatus = TurnState.completed;
        final ctrl = fakeApi.enqueueStream();
        addTearDown(() async {
          if (!ctrl.isClosed) await ctrl.close();
        });

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();
        ctrl.add(const RunStartedEvent('run-1'));
        await tester.pump();

        await tester.pump(const Duration(seconds: 15));
        await tester.pumpAndSettle();

        final after = notifier.state.value!;
        final stillInDots = after.messages.any(
          (m) =>
              m.id == 'assistant-streaming' &&
              m.isStreaming &&
              m.content.isEmpty,
        );
        expect(
          stillInDots,
          isFalse,
          reason: 'probe=completed must clear the dots placeholder',
        );
        expect(fakeApi.turnStatusCalls, isNotEmpty);
      },
    );

    testWidgets('probe=errored triggers recovery and clears the placeholder', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      fakeApi.nextTurnStatus = TurnState.errored;
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();

      await tester.pump(const Duration(seconds: 15));
      await tester.pumpAndSettle();

      final after = notifier.state.value!;
      final stillInDots = after.messages.any(
        (m) =>
            m.id == 'assistant-streaming' && m.isStreaming && m.content.isEmpty,
      );
      expect(
        stillInDots,
        isFalse,
        reason: 'probe=errored must clear the dots placeholder',
      );
    });

    testWidgets(
      'probe=unknown triggers recovery (server no longer has the turn)',
      (tester) async {
        final fakeApi = _FakeApiClient();
        fakeApi.nextTurnStatus = TurnState.unknown;
        final ctrl = fakeApi.enqueueStream();
        addTearDown(() async {
          if (!ctrl.isClosed) await ctrl.close();
        });

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();
        ctrl.add(const RunStartedEvent('run-1'));
        await tester.pump();

        await tester.pump(const Duration(seconds: 15));
        await tester.pumpAndSettle();

        final after = notifier.state.value!;
        final stillInDots = after.messages.any(
          (m) =>
              m.id == 'assistant-streaming' &&
              m.isStreaming &&
              m.content.isEmpty,
        );
        expect(
          stillInDots,
          isFalse,
          reason: 'probe=unknown must clear the dots placeholder',
        );
      },
    );

    testWidgets('probe failure (null return) falls back to legacy recovery', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      fakeApi.nextTurnStatus = null; // simulate probe failing
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });

      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();

      await tester.pump(const Duration(seconds: 15));
      await tester.pumpAndSettle();

      final after = notifier.state.value!;
      final stillInDots = after.messages.any(
        (m) =>
            m.id == 'assistant-streaming' && m.isStreaming && m.content.isEmpty,
      );
      expect(
        stillInDots,
        isFalse,
        reason:
            'probe failure must not leave the UI in dots — fall back to '
            'legacy recovery so the user gets out of the wait state',
      );
    });

    testWidgets(
      'stream without RunStarted (no run id) bypasses probe, runs recovery',
      (tester) async {
        // No RunStartedEvent ever fires → `_currentRunId` stays null and the
        // probe is skipped. The watchdog must still resolve via recovery.
        final fakeApi = _FakeApiClient();
        // nextTurnStatus left null; should not be consulted anyway.
        final ctrl = fakeApi.enqueueStream();
        addTearDown(() async {
          if (!ctrl.isClosed) await ctrl.close();
        });

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();
        // Intentionally no RunStartedEvent.

        await tester.pump(const Duration(seconds: 15));
        await tester.pumpAndSettle();

        final after = notifier.state.value!;
        final stillInDots = after.messages.any(
          (m) =>
              m.id == 'assistant-streaming' &&
              m.isStreaming &&
              m.content.isEmpty,
        );
        expect(stillInDots, isFalse);
        expect(
          fakeApi.turnStatusCalls,
          isEmpty,
          reason: 'no run id → probe must not be called',
        );
      },
    );
  });

  // -- Salvaged from PR #809 fix (2): stalled recovery vs concurrent stream --
  //
  // The fix adds `if (latest.isSending) return;` immediately before the
  // `state = AsyncData(...)` write in _recoverStalledStream's success path.
  // This defends a real race: between recovery's onTimeout firing and its
  // `await getConversation(...)` resolving, the queue-drain loop can start
  // a new stream for the next pending message. Without the guard, the
  // recovery's state write silently wipes the new stream's placeholder.
  //
  // A focused integration test for this race needs a fake conversations
  // API that resolves getConversation deterministically — the current
  // _FakeApiClient only mocks streamMessages and sendVoiceMessage, so
  // getConversation goes through the real Dio path and hangs the test on
  // the connectTimeout (15 s real time). Building the deeper mock is its
  // own follow-up. The fix itself is small, additive, and well-commented
  // in source; the trace lives in this change's commit message + the
  // openspec/changes/sse-keepalive/ analysis.

  // -- Salvaged from PR #809 fix (3): attemptReconnect drains queue ---------
  //
  // attemptReconnect's early-return (`if (!_needsReconnect) return;`)
  // historically skipped any queue drain when there was no interrupted
  // stream to recover. If the drain stopped while messages remained in
  // pendingQueue (rare, but possible via any control-flow path that
  // exits _drainQueue with a non-empty queue), foregrounding the app
  // would not recover. The salvaged fix adds a defence-in-depth net:
  // when !_needsReconnect && !_draining && queue.isNotEmpty, kick the
  // drain anyway.

  group(
    'ChatNotifier.attemptReconnect — queue drain safety net on foreground',
    () {
      testWidgets(
        '13.1: attemptReconnect with no queue and no interrupted stream is a no-op',
        (tester) async {
          final fakeApi = _FakeApiClient();
          final notifier = await _pumpTestApp(tester, fakeApi);

          // Baseline state: no queue, no streaming.
          await tester.pumpAndSettle();
          final before = notifier.state.value!;
          expect(before.pendingQueue, isEmpty);
          expect(before.isSending, isFalse);

          // Reconnect call should do nothing.
          await notifier.attemptReconnect();
          await tester.pumpAndSettle();

          final after = notifier.state.value!;
          expect(after.pendingQueue, isEmpty);
          expect(after.isSending, isFalse);
          // No new messages should have appeared.
          expect(
            after.messages,
            isEmpty,
            reason: 'attemptReconnect on a clean state must be a true no-op',
          );
        },
      );
    },
  );

  // -- Coverage gap #2: TimeoutException from byte heartbeat ----------------
  //
  // The 90-second byte-level watchdog (`withHeartbeatTimeout` in
  // api_client.dart) injects a TimeoutException into the SSE event
  // stream when no bytes arrive for the configured window. That
  // exception flows up to _streamMessage's catch block, which classifies
  // it via _isTransientError (`if (error is TimeoutException) return true`).
  //
  // The classification is unit-tested at the heartbeat layer but the
  // chat_provider's reaction — same retry/defer machinery as for
  // HttpException — was never exercised. This pins it.

  group(
    'ChatNotifier — heartbeat TimeoutException is treated as transient',
    () {
      testWidgets('14.1: TimeoutException after a run ID defers reconnect '
          '(same path as HttpException)', (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();

        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();

        await tester.runAsync(() async {
          // Run ID captured so deferred reconnect is possible.
          ctrl.add(const RunStartedEvent('run-timeout'));
          await Future<void>.delayed(Duration.zero);
          // Byte heartbeat watchdog elapsed — surfaces as a
          // TimeoutException on the event stream.
          ctrl.addError(
            TimeoutException(
              'No data received within 0:01:30.000000',
              const Duration(seconds: 90),
            ),
          );
          await ctrl.close();
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        // Pump through the 3 retry delays (1 + 2 + 4 = 7 s).
        await tester.pump(const Duration(seconds: 2));
        await tester.pump(const Duration(seconds: 3));
        await tester.pump(const Duration(seconds: 5));
        await tester.pumpAndSettle();

        final chatState = notifier.state.value!;
        expect(
          chatState.error,
          isNull,
          reason:
              'transient TimeoutException with run ID should NOT show '
              'an error to the user — it should silently defer',
        );
        expect(
          notifier.needsReconnect,
          isTrue,
          reason: 'TimeoutException should flag for reconnect on resume',
        );
        expect(
          chatState.isSending,
          isFalse,
          reason:
              'after retries exhausted, isSending must be cleared so the '
              'composer is interactive again',
        );
        final userMsg = chatState.messages.firstWhere((m) => m.isUser);
        expect(
          userMsg.status,
          MessageStatus.sending,
          reason:
              'user message stays "sending" until the deferred reconnect '
              'replays the run',
        );
      });

      testWidgets(
        '14.2: TimeoutException without a run ID shows friendly error',
        (tester) async {
          final fakeApi = _FakeApiClient();
          final ctrl = fakeApi.enqueueStream();

          final notifier = await _pumpTestApp(tester, fakeApi);

          unawaited(notifier.sendMessage('hello'));
          await tester.pump();

          await tester.runAsync(() async {
            // Heartbeat fires before any RunStartedEvent arrived.
            ctrl.addError(
              TimeoutException(
                'No data received within 0:01:30.000000',
                const Duration(seconds: 90),
              ),
            );
            await ctrl.close();
            await Future<void>.delayed(Duration.zero);
            await Future<void>.delayed(Duration.zero);
          });
          await tester.pumpAndSettle();

          final chatState = notifier.state.value!;
          expect(
            chatState.error,
            isNotNull,
            reason:
                'no run ID means no replay possibility — must show error '
                'instead of silent defer',
          );
          expect(
            chatState.error,
            isNot(contains('TimeoutException')),
            reason: 'must not expose the raw exception type to the user',
          );
          expect(
            chatState.error!.toLowerCase(),
            anyOf(contains('connection'), contains('timed out')),
            reason: 'should surface a friendly connection / timeout message',
          );
          expect(
            notifier.needsReconnect,
            isFalse,
            reason: 'cannot defer without a run ID',
          );
          expect(
            chatState.isSending,
            isFalse,
            reason: 'isSending cleared so user can retry',
          );
        },
      );
    },
  );

  // -- Coverage gap #5: legitimate slow stream blocks drain (the user-      --
  //    visible symptom motivating PR #809). Documents current contract.    --
  //
  // When a stream legitimately takes a long time (e.g. a slow tool call),
  // the queue does NOT advance until the stream completes. This is the
  // current — and intentional — behaviour. The user-experience problem
  // ("my queued message felt stuck") is addressed by the
  // chat-stream-progress-ux + turn-status-endpoint OpenSpec changes,
  // which add visible queue affordances and explicit cancellation. This
  // test pins the current contract so when those changes alter behaviour,
  // the regression is conscious and visible.

  group(
    'ChatNotifier — long legitimate stream holds the drain (current contract)',
    () {
      testWidgets(
        '15.1: a stream that emits a token then stays open holds the queued '
        'second message in pendingQueue until the first stream completes',
        (tester) async {
          final fakeApi = _FakeApiClient();
          // ctrl1: emits a token (sawProgress=true) then stays open for
          // an extended period — simulating a slow tool call with the
          // initial pre-progress fence already crossed.
          final ctrl1 = fakeApi.enqueueStream();
          addTearDown(() async {
            if (!ctrl1.isClosed) await ctrl1.close();
          });
          // ctrl2: queued message — fully prepared, will be consumed
          // when the drain advances to it.
          fakeApi.enqueueStream()
            ..add(const TokenEvent('reply2-token'))
            ..add(const DoneEvent(role: 'assistant', content: 'reply2-token'))
            ..close();

          final notifier = await _pumpTestApp(tester, fakeApi);

          unawaited(notifier.sendMessage('slow tool turn'));
          await tester.pump();
          // First token arrives — sawProgress=true now, the 12s
          // initial-stall watchdog becomes a no-op for subsequent
          // silence on this stream.
          ctrl1.add(const RunStartedEvent('run-slow'));
          ctrl1.add(const TokenEvent('Calling tool... '));
          await tester.pump();

          // User types a follow-up while the slow tool is running.
          unawaited(notifier.sendMessage('do this next'));
          await tester.pump();
          expect(
            notifier.state.value!.pendingQueue.map((p) => p.text),
            contains('do this next'),
            reason: 'second message must be queued, not dropped',
          );

          // Advance simulated time well past the 12 s initial-stall
          // watchdog. sawProgress=true makes that timeout a no-op for
          // ongoing silence on this stream. The 90 s byte heartbeat
          // operates on a byte-level stream not driven by this event-
          // level controller, so it does NOT fire here either. The
          // stream is "legitimately busy" from the client's view.
          await tester.pump(const Duration(seconds: 60));
          await tester.pump();

          // Current contract: the queued message stays queued. The
          // user sees no progress — that's the UX problem the future
          // chat-stream-progress-ux change will address. We pin the
          // current behaviour so future changes are conscious.
          expect(
            notifier.state.value!.pendingQueue.map((p) => p.text),
            contains('do this next'),
            reason:
                'CURRENT CONTRACT: a legitimate long-running stream holds '
                'the queue. The user-visible symptom that motivated PR '
                '#809 is this state — invisible to the user. When the '
                'chat-stream-progress-ux change ships and this test fails, '
                'the failure must be intentional (visible queue + skip '
                'affordance) and the test should be updated.',
          );
          expect(
            notifier.state.value!.isSending,
            isTrue,
            reason: 'first turn is still streaming',
          );

          // The follow-up assertion ("when ctrl1 completes, the drain
          // advances and processes the queued message") is what the
          // overall contract guarantees, but verifying it in this test
          // would couple to microtask ordering after ctrl1.close().
          // The existing "ChatNotifier.sendMessage — queue behaviour"
          // tests (group 7) cover the drain-advances-after-Done case
          // exhaustively. The unique contract this test pins is
          // the held-queue state during the slow stream above.
        },
      );
    },
  );

  // -- Phase A of chat-stream-progress-ux: currentTurnStatus derived state --
  //
  // The UI's in-flight progress card consumes a `currentTurnStatus`
  // snapshot exposed on ChatState. The snapshot tracks the active turn's
  // runId, the kind of its most recent SSE event, the wall-clock
  // timestamp of that event, and the name of any currently-active tool.
  // These tests pin the state-machine transitions per SSE event kind so
  // the widget layer (Phase B) can rely on a stable observable surface.

  group('ChatNotifier — currentTurnStatus derived state', () {
    testWidgets('initial state has no currentTurnStatus', (tester) async {
      final fakeApi = _FakeApiClient();
      final notifier = await _pumpTestApp(tester, fakeApi);
      await tester.pump();

      expect(
        notifier.state.value!.currentTurnStatus,
        isNull,
        reason: 'No turn in flight at startup — snapshot should be null',
      );
    });

    testWidgets('RunStartedEvent populates currentTurnStatus.runId', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-abc'));
      await tester.pump();

      final snap = notifier.state.value!.currentTurnStatus;
      expect(snap, isNotNull, reason: 'RunStartedEvent should open a snapshot');
      expect(snap!.runId, 'run-abc');
      expect(snap.lastEventKind, TurnEventKind.runStarted);
      expect(snap.currentToolName, isNull);

      // Close the stream so the source's `.timeout()` watchdog is
      // cancelled and the test exits without a pending-timer assertion.
      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets(
      'TokenEvent updates lastEventKind to token and clears tool name',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();
        addTearDown(() async {
          if (!ctrl.isClosed) await ctrl.close();
        });
        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();
        ctrl.add(const RunStartedEvent('run-1'));
        await tester.pump();
        // A tool result first — populates currentToolName.
        ctrl.add(const ToolResultEvent(toolName: 'fetch_url', status: 'ok'));
        await tester.pump();
        expect(
          notifier.state.value!.currentTurnStatus!.currentToolName,
          'fetch_url',
        );
        // Now the assistant resumes producing tokens — tool name cleared,
        // lastEventKind transitions to token.
        ctrl.add(const TokenEvent('hi '));
        await tester.pump();

        final snap = notifier.state.value!.currentTurnStatus!;
        expect(snap.lastEventKind, TurnEventKind.token);
        expect(
          snap.currentToolName,
          isNull,
          reason:
              'TokenEvent means the assistant resumed text generation; '
              'the tool name from the previous tool_result should clear',
        );

        await ctrl.close();
        await tester.pumpAndSettle();
      },
    );

    testWidgets(
      'ToolResultEvent updates lastEventKind=toolResult and currentToolName',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();
        addTearDown(() async {
          if (!ctrl.isClosed) await ctrl.close();
        });
        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();
        ctrl.add(const RunStartedEvent('run-1'));
        await tester.pump();
        ctrl.add(const ToolResultEvent(toolName: 'fetch_url', status: 'ok'));
        await tester.pump();

        final snap = notifier.state.value!.currentTurnStatus!;
        expect(snap.lastEventKind, TurnEventKind.toolResult);
        expect(snap.currentToolName, 'fetch_url');

        await ctrl.close();
        await tester.pumpAndSettle();
      },
    );

    testWidgets('ThinkingEvent updates lastEventKind=thinking', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();
      ctrl.add(const ThinkingEvent('reasoning...'));
      await tester.pump();

      expect(
        notifier.state.value!.currentTurnStatus!.lastEventKind,
        TurnEventKind.thinking,
      );

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('StatusEvent updates lastEventKind=status', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();
      ctrl.add(const StatusEvent('Working...'));
      await tester.pump();

      expect(
        notifier.state.value!.currentTurnStatus!.lastEventKind,
        TurnEventKind.status,
      );

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets(
      'SubagentStartedEvent updates lastEventKind=subagentStarted and tool name',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl = fakeApi.enqueueStream();
        addTearDown(() async {
          if (!ctrl.isClosed) await ctrl.close();
        });
        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('hello'));
        await tester.pump();
        ctrl.add(const RunStartedEvent('run-1'));
        await tester.pump();
        ctrl.add(
          const SubagentStartedEvent(agentId: 'a1', task: 'analyze data'),
        );
        await tester.pump();

        final snap = notifier.state.value!.currentTurnStatus!;
        expect(snap.lastEventKind, TurnEventKind.subagentStarted);
        expect(snap.currentToolName, 'analyze data');

        await ctrl.close();
        await tester.pumpAndSettle();
      },
    );

    testWidgets('DoneEvent clears currentTurnStatus', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      ctrl.add(const TokenEvent('hi'));
      ctrl.add(const DoneEvent(role: 'assistant', content: 'hi'));
      await tester.pumpAndSettle();

      expect(
        notifier.state.value!.currentTurnStatus,
        isNull,
        reason: 'DoneEvent ends the turn — snapshot must be cleared',
      );
    });

    testWidgets('transient stream error with run ID clears currentTurnStatus', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      await tester.runAsync(() async {
        ctrl.add(const RunStartedEvent('run-defer'));
        await Future<void>.delayed(Duration.zero);
        ctrl.addError(
          const HttpException('Connection closed while receiving data'),
        );
        await ctrl.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      // Pump through 3 retry delays (1+2+4 s).
      await tester.pump(const Duration(seconds: 2));
      await tester.pump(const Duration(seconds: 3));
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();

      expect(
        notifier.state.value!.currentTurnStatus,
        isNull,
        reason: 'After retries exhaust, the snapshot must be cleared',
      );
    });

    testWidgets('lastEventAt is monotonically updated on each event', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hello'));
      await tester.pump();
      ctrl.add(const RunStartedEvent('run-1'));
      await tester.pump();
      final t1 = notifier.state.value!.currentTurnStatus!.lastEventAt;

      // Real wall-clock delay so the second timestamp differs.
      await tester.runAsync(() async {
        await Future<void>.delayed(const Duration(milliseconds: 10));
      });
      ctrl.add(const TokenEvent('hi'));
      await tester.pump();
      final t2 = notifier.state.value!.currentTurnStatus!.lastEventAt;

      expect(
        t2.isAfter(t1) || t2.isAtSameMomentAs(t1),
        isTrue,
        reason: 'lastEventAt must not regress between events',
      );

      await ctrl.close();
      await tester.pumpAndSettle();
    });
  });

  // -- Phase C of chat-stream-progress-ux: removeFromQueue ------------------

  group('ChatNotifier.removeFromQueue', () {
    testWidgets('removes the entry at the given index', (tester) async {
      final fakeApi = _FakeApiClient();
      final notifier = await _pumpTestApp(tester, fakeApi);
      await tester.pump();

      // Seed the queue with three entries via state mutation (this
      // matches what `sendMessage` would have done; no public mutator
      // exists for direct injection, but state is settable from inside
      // the notifier via the AsyncNotifier protected setter — which
      // tests cannot reach. So we drive through sendMessage instead).
      //
      // First message goes straight to streaming via the drain;
      // subsequent ones land in pendingQueue. We enqueue 3 streams that
      // each immediately complete so the drain consumes them in order.
      // To leave items in the queue, we send 3 messages while the FIRST
      // stream is intentionally held open.
      final ctrl1 = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl1.isClosed) await ctrl1.close();
      });
      // Streams for messages 2 + 3 that the drain will eventually pop;
      // we don't actually advance to them in this test.
      fakeApi.enqueueStream();
      fakeApi.enqueueStream();

      unawaited(notifier.sendMessage('a'));
      await tester.pump();
      ctrl1.add(const RunStartedEvent('run-1'));
      await tester.pump();
      unawaited(notifier.sendMessage('b'));
      await tester.pump();
      unawaited(notifier.sendMessage('c'));
      await tester.pump();

      expect(notifier.state.value!.pendingQueue.map((p) => p.text).toList(), [
        'b',
        'c',
      ], reason: 'precondition: a is streaming, b and c are queued in order');

      // Remove the middle of the queue (index 0 in the visible queue,
      // since `a` is no longer in pendingQueue — it's actively streaming).
      notifier.removeFromQueue(0);
      await tester.pump();

      expect(notifier.state.value!.pendingQueue.map((p) => p.text).toList(), [
        'c',
      ], reason: 'removeFromQueue(0) drops "b" but leaves "c"');

      await ctrl1.close();
      await tester.pumpAndSettle();
    });

    testWidgets('out-of-bounds index is a no-op', (tester) async {
      final fakeApi = _FakeApiClient();
      final notifier = await _pumpTestApp(tester, fakeApi);
      await tester.pump();

      // Empty queue — any index is out of bounds.
      notifier.removeFromQueue(0);
      notifier.removeFromQueue(-1);
      notifier.removeFromQueue(99);
      await tester.pump();

      expect(notifier.state.value!.pendingQueue, isEmpty);
    });
  });

  // -- Phase D of chat-stream-progress-ux: isReconnecting flag --------------
  //
  // The full "flag flips true during recovery, false on completion" path
  // is implicitly verified by the widget test (which renders the banner
  // against a forced isReconnecting=true state) and by inspection of the
  // try/finally block in `attemptReconnect`. The during-recovery
  // assertion requires mocking `api.conversations.getConversation` —
  // the current _FakeApiClient does not, so the integration test hangs
  // on Dio's 15 s connectTimeout. Same limitation as PR #818's fix-2
  // positive case. Source-documented; defer the integration test to
  // the deeper-mock follow-up.

  group('ChatNotifier.attemptReconnect — isReconnecting flag', () {
    testWidgets(
      'routine resume (no needsReconnect, no queue) keeps isReconnecting false',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final notifier = await _pumpTestApp(tester, fakeApi);
        await tester.pumpAndSettle();

        expect(notifier.needsReconnect, isFalse);
        expect(notifier.state.value!.isReconnecting, isFalse);

        await notifier.attemptReconnect();
        await tester.pumpAndSettle();

        expect(
          notifier.state.value!.isReconnecting,
          isFalse,
          reason:
              'no interrupted stream → no banner; the early-return branch '
              'must not flip the flag',
        );
      },
    );
  });

  // -- Characterization: timeline reducers + error mapping -------------------
  //
  // Pin the behaviour of the per-event timeline mutations and the stream
  // error mapping so they can move out of ChatNotifier unchanged.

  group('ChatNotifier — timeline event reducers', () {
    /// Starts a turn, feeds [events], and returns the messages once pumped.
    Future<(ChatNotifier, StreamController<StreamEvent>)> startTurn(
      WidgetTester tester,
      List<StreamEvent> events,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);
      unawaited(notifier.sendMessage('go'));
      await tester.pump();
      for (final e in events) {
        ctrl.add(e);
        await tester.pump();
      }
      return (notifier, ctrl);
    }

    ChatMessage subagentEntry(ChatNotifier n, String id) =>
        n.state.value!.messages.firstWhere(
          (m) =>
              m.timelineType == TimelineEntryType.subagent &&
              m.subagentId == id,
        );

    // The text path (`_streamMessage`) does not handle the subagent
    // token/thinking/tool-result events; only the voice path does.
    testWidgets('voice stream accumulates subagent tokens, thinking, tools', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueVoiceStream();
      addTearDown(() async {
        if (!ctrl.isClosed) await ctrl.close();
      });
      final notifier = await _pumpTestApp(tester, fakeApi);
      unawaited(notifier.sendVoiceMessage(Uint8List(4), 'audio/webm'));
      await tester.pump();
      for (final e in const <StreamEvent>[
        SubagentStartedEvent(agentId: 'a1', task: 'research'),
        SubagentTokenEvent(agentId: 'a1', content: 'foo '),
        SubagentTokenEvent(agentId: 'a1', content: 'bar'),
        SubagentThinkingEvent(agentId: 'a1', content: 'hmm '),
        SubagentThinkingEvent(agentId: 'a1', content: 'ok'),
        SubagentToolResultEvent(
          agentId: 'a1',
          toolName: 'web-search',
          status: 'error',
          arguments: {'q': 'x'},
          result: 'boom',
        ),
        SubagentToolResultEvent(
          agentId: 'a1',
          toolName: 'bash',
          status: 'weird',
        ),
        // Events for an unknown subagent are ignored.
        SubagentTokenEvent(agentId: 'ghost', content: 'nope'),
      ]) {
        ctrl.add(e);
        await tester.pump();
      }

      final entry = subagentEntry(notifier, 'a1');
      expect(entry.subagentContent, 'foo bar');
      expect(entry.subagentThinking, 'hmm ok');
      expect(entry.subagentToolCalls.map((t) => t.toolName), [
        'web-search',
        'bash',
      ]);
      expect(entry.subagentToolCalls.first.status, ToolCallStatus.error);
      expect(entry.subagentToolCalls.first.arguments, {'q': 'x'});
      expect(entry.subagentToolCalls.first.result, 'boom');
      expect(
        entry.subagentToolCalls.last.status,
        ToolCallStatus.ok,
        reason: 'unknown status strings map to ok',
      );
      expect(
        notifier.state.value!.messages.where((m) => m.subagentId == 'ghost'),
        isEmpty,
      );

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('cancelled subagent completion marks the entry stale', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        SubagentStartedEvent(agentId: 'a1', task: 't'),
        SubagentStartedEvent(agentId: 'a2', task: 't'),
        SubagentCompletedEvent(
          agentId: 'a1',
          status: 'cancelled',
          summary: 'stopped',
        ),
        SubagentCompletedEvent(agentId: 'a2', status: 'ok', summary: 'done'),
      ]);

      final cancelled = subagentEntry(notifier, 'a1');
      expect(cancelled.isStale, isTrue);
      expect(cancelled.isStreaming, isFalse);
      expect(cancelled.subagentSummary, 'stopped');
      final ok = subagentEntry(notifier, 'a2');
      expect(ok.isStale, isFalse);
      expect(ok.isStreaming, isFalse);

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('tool entries insert before the streaming placeholder', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        StatusEvent('Calling tool: web-search', toolCallId: 'c1'),
      ]);

      final msgs = notifier.state.value!.messages;
      final toolIdx = msgs.indexWhere(
        (m) => m.timelineType == TimelineEntryType.toolCall,
      );
      final placeholderIdx = msgs.indexWhere(
        (m) => m.id == 'assistant-streaming',
      );
      expect(toolIdx, isNonNegative);
      expect(toolIdx, lessThan(placeholderIdx));
      final record = msgs[toolIdx].toolCalls.single;
      expect(record.toolName, 'web-search');
      expect(record.toolCallId, 'c1');
      expect(record.status, ToolCallStatus.pending);
      expect(msgs[toolIdx].isStreaming, isTrue);
      expect(notifier.state.value!.statusMessage, 'Calling tool: web-search');

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('tool result resolves the pending entry matching toolCallId', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        StatusEvent('Calling tool: bash', toolCallId: 'c1'),
        StatusEvent('Calling tool: bash', toolCallId: 'c2'),
        ToolResultEvent(
          toolName: 'bash',
          status: 'denied',
          toolCallId: 'c1',
          arguments: {'cmd': 'ls'},
          result: 'no',
        ),
      ]);

      final tools = notifier.state.value!.messages
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .toList();
      expect(tools, hasLength(2));
      final first = tools.first.toolCalls.single;
      expect(first.toolCallId, 'c1');
      expect(first.status, ToolCallStatus.denied);
      expect(first.arguments, {'cmd': 'ls'});
      expect(first.result, 'no');
      expect(first.duration, isNotNull);
      expect(tools.first.isStreaming, isFalse);
      expect(tools.last.toolCalls.single.status, ToolCallStatus.pending);
      final last = notifier.state.value!.lastToolResult!;
      expect(last.toolName, 'bash');
      expect(last.status, 'denied');
      expect(last.isSuccess, isFalse);
      expect(notifier.state.value!.statusMessage, isNull);

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('tool result without toolCallId falls back to the tool name', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        StatusEvent('Calling tool: a'),
        StatusEvent('Calling tool: b'),
        ToolResultEvent(toolName: 'a', status: 'ok'),
      ]);

      final tools = notifier.state.value!.messages
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .toList();
      expect(tools.first.toolCalls.single.status, ToolCallStatus.ok);
      expect(tools.last.toolCalls.single.status, ToolCallStatus.pending);

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('orphan tool result inserts a resolved entry', (tester) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        ToolResultEvent(toolName: 'file-read', status: 'error', result: 'x'),
      ]);

      final msgs = notifier.state.value!.messages;
      final toolIdx = msgs.indexWhere(
        (m) => m.timelineType == TimelineEntryType.toolCall,
      );
      expect(
        toolIdx,
        lessThan(msgs.indexWhere((m) => m.id == 'assistant-streaming')),
      );
      final record = msgs[toolIdx].toolCalls.single;
      expect(record.toolName, 'file-read');
      expect(record.status, ToolCallStatus.error);
      expect(record.result, 'x');
      expect(msgs[toolIdx].isStreaming, isFalse);

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('a new timeline entry completes earlier streaming ones', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        ThinkingEvent('hmm'),
        StatusEvent('Calling tool: a'),
      ]);

      final msgs = notifier.state.value!.messages;
      final thinking = msgs.firstWhere(
        (m) => m.timelineType == TimelineEntryType.thinking,
      );
      expect(thinking.isStreaming, isFalse);
      final tool = msgs.firstWhere(
        (m) => m.timelineType == TimelineEntryType.toolCall,
      );
      expect(tool.isStreaming, isTrue);

      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('thinking tokens are fed to the entry token stream', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        ThinkingEvent('one '),
      ]);
      final entry = notifier.state.value!.messages.firstWhere(
        (m) => m.timelineType == TimelineEntryType.thinking,
      );
      expect(entry.thinkingTokenStream, isNotNull);
      final received = <String>[];
      final sub = entry.thinkingTokenStream!.listen(received.add);

      ctrl.add(const ThinkingEvent('two'));
      await tester.pump();
      await tester.runAsync(() => Future<void>.delayed(Duration.zero));

      expect(received, ['two']);
      unawaited(sub.cancel());
      await ctrl.close();
      await tester.pumpAndSettle();
    });

    testWidgets('first answer token marks timeline entries stale', (
      tester,
    ) async {
      final (notifier, ctrl) = await startTurn(tester, const [
        ThinkingEvent('hmm'),
        StatusEvent('Calling tool: a'),
        ToolResultEvent(toolName: 'a', status: 'ok'),
        TokenEvent('answer'),
      ]);

      final msgs = notifier.state.value!.messages;
      final timeline = msgs.where(
        (m) => m.timelineType != TimelineEntryType.message,
      );
      expect(timeline, hasLength(2));
      for (final m in timeline) {
        expect(m.isStale, isTrue);
        expect(m.isStreaming, isFalse);
      }
      expect(
        msgs.firstWhere((m) => m.id == 'assistant-streaming').isStale,
        isFalse,
      );

      await ctrl.close();
      await tester.pumpAndSettle();
    });
  });

  group('ChatNotifier — stream error mapping', () {
    testWidgets('non-transient stream error surfaces as Stream error', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hi'));
      await tester.pump();
      await tester.runAsync(() async {
        ctrl.addError(StateError('kaboom'));
        await ctrl.close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      final s = notifier.state.value!;
      expect(s.error, 'Stream error: Bad state: kaboom');
      expect(s.isSending, isFalse);
      expect(
        s.messages.firstWhere((m) => m.isUser).status,
        MessageStatus.failed,
      );
    });

    testWidgets('transient error without a run id shows the retry hint', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('hi'));
      await tester.pump();
      await tester.runAsync(() async {
        ctrl.addError(const SocketException('Connection reset by peer'));
        await ctrl.close();
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pumpAndSettle();

      expect(
        notifier.state.value!.error,
        'Connection lost — tap retry to resend',
      );
      expect(notifier.needsReconnect, isFalse);
    });
  });

  // -- Stream event parity ----------------------------------------------------
  //
  // The text, voice and replay paths all consume the same SSE event
  // vocabulary; the timeline they build must not depend on which path
  // delivered the events.

  group('ChatNotifier — stream event parity across paths', () {
    final subagentEvents = <StreamEvent>[
      SubagentStartedEvent.fromJson({'agent_id': 'a1', 'task': 'Research'}),
      const SubagentTokenEvent(agentId: 'a1', content: 'partial '),
      const SubagentTokenEvent(agentId: 'a1', content: 'answer'),
      const SubagentThinkingEvent(agentId: 'a1', content: 'hmm'),
      const SubagentToolResultEvent(
        agentId: 'a1',
        toolName: 'web-fetch',
        status: 'ok',
        result: 'page body',
      ),
      const SubagentStatusEvent(agentId: 'a1', message: 'working'),
    ];

    final timelineEvents = <StreamEvent>[
      const ThinkingEvent('reasoning'),
      const StatusEvent('Running tool: web-search'),
      const ToolResultEvent(toolName: 'web-search', status: 'ok'),
      ...subagentEvents,
      SubagentCompletedEvent.fromJson({
        'agent_id': 'a1',
        'status': 'ok',
        'summary': 'done researching',
      }),
    ];

    Future<void> deliver(
      WidgetTester tester,
      StreamController<StreamEvent> ctrl,
      Iterable<StreamEvent> events,
    ) async {
      await tester.runAsync(() async {
        for (final event in events) {
          ctrl.add(event);
          await Future<void>.delayed(Duration.zero);
        }
      });
      await tester.pump();
    }

    testWidgets('text path: subagent token, thinking and tool-result events '
        'reach the subagent timeline entry', (tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('go'));
      await tester.pump();
      await deliver(tester, ctrl, subagentEvents);

      final entry = notifier.state.value!.messages.firstWhere(
        (m) => m.timelineType == TimelineEntryType.subagent,
      );
      expect(entry.subagentContent, 'partial answer');
      expect(entry.subagentThinking, 'hmm');
      expect(entry.subagentToolCalls.map((c) => c.toolName), ['web-fetch']);

      ctrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();
    });

    testWidgets(
      'voice path: subagent events reach the subagent timeline entry',
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
        await deliver(tester, ctrl, subagentEvents);

        final entry = notifier.state.value!.messages.firstWhere(
          (m) => m.timelineType == TimelineEntryType.subagent,
        );
        expect(entry.subagentContent, 'partial answer');
        expect(entry.subagentThinking, 'hmm');
        expect(entry.subagentToolCalls.map((c) => c.toolName), ['web-fetch']);

        ctrl
          ..add(const DoneEvent(role: 'assistant', content: 'done'))
          ..close();
        await tester.pumpAndSettle();
      },
    );

    testWidgets(
      'replay path: thinking and subagent events reach the timeline',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final ctrl1 = fakeApi.enqueueStream();
        fakeApi.enqueueReplayStream()
          ..add(const ThinkingEvent('reasoning'))
          ..add(SubagentStartedEvent.fromJson({'agent_id': 'a1', 'task': 'T'}))
          ..add(const SubagentTokenEvent(agentId: 'a1', content: 'tok'))
          ..add(const DoneEvent(role: 'assistant', content: 'ok'))
          ..close();
        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('ping'));
        await tester.pump();
        await tester.runAsync(() async {
          ctrl1.add(const RunStartedEvent('run-abc'));
          await Future<void>.delayed(Duration.zero);
          ctrl1.addError(const SocketException('network drop'));
          await ctrl1.close();
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pump(const Duration(seconds: 2));
        await tester.pumpAndSettle();

        final msgs = notifier.state.value!.messages;
        expect(
          msgs.where((m) => m.timelineType == TimelineEntryType.thinking),
          isNotEmpty,
          reason: 'replayed ThinkingEvent should produce a thinking entry',
        );
        final entry = msgs.firstWhere(
          (m) => m.timelineType == TimelineEntryType.subagent,
        );
        expect(entry.subagentContent, 'tok');
      },
    );

    testWidgets('text and voice paths build identical timelines', (
      tester,
    ) async {
      List<String> snapshot(ChatState s) => [
        for (final m in s.messages)
          if (m.timelineType != TimelineEntryType.message)
            [
              m.timelineType.name,
              m.thinkingContent,
              m.subagentId,
              m.subagentSummary,
              m.subagentContent,
              m.subagentThinking,
              m.subagentToolCalls.map((c) => '${c.toolName}:${c.status}'),
              m.toolCalls.map((c) => '${c.toolName}:${c.status}'),
              m.isStreaming,
            ].join('|'),
      ];

      final textApi = _FakeApiClient();
      final textCtrl = textApi.enqueueStream();
      final textNotifier = await _pumpTestApp(tester, textApi);
      unawaited(textNotifier.sendMessage('go'));
      await tester.pump();
      await deliver(tester, textCtrl, timelineEvents);
      final textSnapshot = snapshot(textNotifier.state.value!);
      textCtrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();

      // Tear down the first scope so the voice run starts from a fresh state.
      await tester.pumpWidget(const SizedBox());

      final voiceApi = _FakeApiClient();
      final voiceCtrl = voiceApi.enqueueVoiceStream();
      final voiceNotifier = await _pumpTestApp(tester, voiceApi);
      unawaited(
        voiceNotifier.sendVoiceMessage(
          Uint8List.fromList([1, 2, 3]),
          'audio/webm',
        ),
      );
      await tester.pump();
      await deliver(tester, voiceCtrl, timelineEvents);
      final voiceSnapshot = snapshot(voiceNotifier.state.value!);
      voiceCtrl
        ..add(const DoneEvent(role: 'assistant', content: 'done'))
        ..close();
      await tester.pumpAndSettle();

      expect(textSnapshot, isNotEmpty);
      expect(voiceSnapshot, textSnapshot);
    });
  });

  // -- agent_error reconciliation ----------------------------------------------
  //
  // The terminal `agent_error` frame must reconcile the turn the same way on
  // the text, voice and replay paths.

  group('ChatNotifier — agent_error reconciliation', () {
    for (final path in ['text', 'voice', 'replay']) {
      Future<({ChatNotifier notifier, _FakeApiClient api})> run(
        WidgetTester tester,
        List<StreamEvent> events,
      ) async {
        final fakeApi = _FakeApiClient();
        final ChatNotifier notifier;
        switch (path) {
          case 'text':
            final ctrl = fakeApi.enqueueStream();
            notifier = await _pumpTestApp(tester, fakeApi);
            unawaited(notifier.sendMessage('go'));
            await tester.pump();
            await tester.runAsync(() async {
              for (final e in events) {
                ctrl.add(e);
                await Future<void>.delayed(Duration.zero);
              }
              await ctrl.close();
              await Future<void>.delayed(Duration.zero);
            });
          case 'voice':
            final ctrl = fakeApi.enqueueVoiceStream();
            notifier = await _pumpTestApp(tester, fakeApi);
            unawaited(
              notifier.sendVoiceMessage(
                Uint8List.fromList([1, 2, 3]),
                'audio/webm',
              ),
            );
            await tester.pump();
            await tester.runAsync(() async {
              for (final e in events) {
                ctrl.add(e);
                await Future<void>.delayed(Duration.zero);
              }
              await ctrl.close();
              await Future<void>.delayed(Duration.zero);
            });
          default:
            final ctrl1 = fakeApi.enqueueStream();
            final replay = fakeApi.enqueueReplayStream();
            for (final e in events) {
              replay.add(e);
            }
            unawaited(replay.close());
            notifier = await _pumpTestApp(tester, fakeApi);
            unawaited(notifier.sendMessage('go'));
            await tester.pump();
            await tester.runAsync(() async {
              ctrl1.add(const RunStartedEvent('run-abc'));
              await Future<void>.delayed(Duration.zero);
              ctrl1.addError(const SocketException('network drop'));
              await ctrl1.close();
              await Future<void>.delayed(Duration.zero);
              await Future<void>.delayed(Duration.zero);
            });
            await tester.pump(const Duration(seconds: 2));
        }
        // Let the replay backoff run out so no retry timer outlives the test.
        await tester.pump(const Duration(seconds: 10));
        await tester.pumpAndSettle();
        return (notifier: notifier, api: fakeApi);
      }

      const cancelled = AgentErrorEvent(
        reason: AgentErrorReason.cancelled,
        message: 'Turn cancelled',
        partialContent: 'The capital',
        messageId: 'M',
      );

      testWidgets('$path: cancelled keeps the partial as a stopped message', (
        tester,
      ) async {
        final r = await run(tester, [
          const TokenEvent('The capital'),
          cancelled,
        ]);

        final s = r.notifier.state.value!;
        final assistant = s.messages.where((m) => !m.isUser).single;
        expect(assistant.id, 'M');
        expect(assistant.content, 'The capital');
        expect(assistant.status, MessageStatus.stopped);
        expect(s.messages.firstWhere((m) => m.isUser).status, MessageStatus.ok);
        expect(s.error, isNull);
        expect(s.isSending, isFalse);
      });

      testWidgets('$path: cancelled with the _Stopped._ placeholder', (
        tester,
      ) async {
        final r = await run(tester, [
          const AgentErrorEvent(
            reason: AgentErrorReason.cancelled,
            message: 'Turn cancelled',
            partialContent: '_Stopped._',
            messageId: 'M',
          ),
        ]);

        final assistant = r.notifier.state.value!.messages
            .where((m) => !m.isUser)
            .single;
        expect(assistant.id, 'M');
        expect(assistant.content, '_Stopped._');
        expect(assistant.status, MessageStatus.stopped);
      });

      testWidgets('$path: cancelled before the user message was persisted', (
        tester,
      ) async {
        final r = await run(tester, [
          const AgentErrorEvent(
            reason: AgentErrorReason.cancelled,
            message: 'Turn cancelled',
            userMessagePersisted: false,
          ),
        ]);

        final s = r.notifier.state.value!;
        expect(s.messages.where((m) => !m.isUser), isEmpty);
        expect(s.messages.single.status, MessageStatus.failed);
        expect(s.error, isNull);
        expect(s.isSending, isFalse);
      });

      testWidgets('$path: error fails the user message and surfaces it', (
        tester,
      ) async {
        final r = await run(tester, [
          const AgentErrorEvent(
            reason: AgentErrorReason.error,
            message: 'LLM timeout',
          ),
        ]);

        final s = r.notifier.state.value!;
        expect(s.messages.where((m) => !m.isUser), isEmpty);
        expect(s.messages.single.status, MessageStatus.failed);
        expect(s.error, 'LLM timeout');
        expect(s.isSending, isFalse);
      });
    }

    testWidgets('a cancelled frame during replay is not retried', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      fakeApi.enqueueReplayStream()
        ..add(
          const AgentErrorEvent(
            reason: AgentErrorReason.cancelled,
            message: 'Turn cancelled',
            partialContent: 'part',
            messageId: 'M',
          ),
        )
        ..close();
      final notifier = await _pumpTestApp(tester, fakeApi);

      unawaited(notifier.sendMessage('go'));
      await tester.pump();
      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-abc'));
        await Future<void>.delayed(Duration.zero);
        ctrl1.addError(const SocketException('network drop'));
        await ctrl1.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump(const Duration(seconds: 2));
      await tester.pumpAndSettle(const Duration(seconds: 10));

      expect(fakeApi.replayCallCount, 1);
      expect(notifier.needsReconnect, isFalse);
    });
  });

  group('loadConversation', () {
    testWidgets('shows a cancelled reply as stopped', (tester) async {
      final fakeApi = _FakeApiClient();
      fakeApi.conversationsFake.serverMessages = [
        ('u1', 'user', 'go'),
        ('a1', 'assistant', 'The capital'),
        ('a2', 'assistant', 'Done'),
      ];
      fakeApi.conversationsFake.finishReasons['a1'] = 'cancelled';
      final notifier = await _pumpTestApp(tester, fakeApi);

      await notifier.loadConversation('conv-1');
      await tester.pump();

      final messages = notifier.state.value!.messages;
      expect(messages.map((m) => m.status), [
        MessageStatus.ok,
        MessageStatus.stopped,
        MessageStatus.ok,
      ]);
    });

    testWidgets('finds a pending interaction and watches its run', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final createdAt = DateTime.parse('2026-05-16T00:01:30Z');
      fakeApi.conversationsFake.serverMessageSummaries = [
        MessageSummary(
          (b) => b
            ..id = 'u1'
            ..role = 'user'
            ..content = 'go'
            ..createdAt = DateTime.parse('2026-05-16T00:01:00Z')
            ..turn = 1
            ..ttsAvailable = false,
        ),
        MessageSummary(
          (b) => b
            ..id = 'a1'
            ..role = 'assistant'
            ..content = ''
            ..createdAt = DateTime.parse('2026-05-16T00:01:10Z')
            ..turn = 1
            ..ttsAvailable = false
            ..toolCalls = ListBuilder<ToolCallSummary>([
              ToolCallSummary(
                (t) => t
                  ..name = 'ask-user'
                  ..status = 'pending'
                  ..toolCallId = 'call-q',
              ),
            ]),
        ),
      ];
      fakeApi.pendingInteractions = [
        InteractionDetail(
          (b) => b
            ..interactionId = 'int-q'
            ..conversationId = 'conv-1'
            ..runId = 'run-7'
            ..kind = 'question'
            ..status = InteractionStatusView.pending
            ..rendersCard = true
            ..toolCallId = 'call-q'
            ..createdAt = createdAt
            ..expiresAt = createdAt.add(const Duration(minutes: 10))
            ..request = JsonObject({
              'questions': [
                {
                  'id': 'q1',
                  'prompt': 'Which branch?',
                  'options': [
                    {'id': 'o1', 'label': 'main'},
                  ],
                  'multiple': false,
                  'free_text': false,
                },
              ],
            }),
        ),
      ];
      fakeApi.conversationsFake.serverInteractions =
          fakeApi.pendingInteractions;
      final replay = fakeApi.enqueueReplayStream();
      final notifier = await _pumpTestApp(tester, fakeApi);

      await notifier.loadConversation('conv-1');
      await tester.pump();

      expect(fakeApi.pendingListCalls, 1);
      expect(
        fakeApi.interactionStatusQueries.single,
        InteractionStatusView.pending,
      );
      final entry = notifier.state.value!.messages.firstWhere(
        (m) => m.timelineType == TimelineEntryType.toolCall,
      );
      final record = entry.toolCalls.single.interaction!;
      expect(record.id, 'int-q');
      expect(record.kind, InteractionKind.question);
      expect(record.status, InteractionStatus.pending);
      expect(
        notifier.state.value!.currentTurnStatus?.awaitingLabel,
        'Waiting for your answer',
      );
      expect(notifier.state.value!.currentTurnStatus?.runId, 'run-7');
      expect(fakeApi.replayCalls.single.runId, 'run-7');
      expect(fakeApi.replayCalls.single.since, 0);

      await tester.runAsync(() async {
        replay.add(
          InteractionResolvedEvent(
            interactionId: 'int-q',
            kind: 'question',
            outcome: InteractionOutcome.resolved,
            resolvedAt: createdAt.add(const Duration(seconds: 20)),
            response: const {
              'answers': [
                {
                  'question_id': 'q1',
                  'selected_option_ids': ['o1'],
                },
              ],
            },
          ),
        );
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();

      expect(
        entry.toolCalls.single.interaction?.status,
        InteractionStatus.resolved,
      );
      expect(
        entry.toolCalls.single.interaction?.answers.single.selectedOptionIds,
        ['o1'],
      );
      unawaited(replay.close());
    });
  });

  group('stopTurn', () {
    const cancelledFrame = AgentErrorEvent(
      reason: AgentErrorReason.cancelled,
      message: 'Turn cancelled',
      partialContent: 'part',
      messageId: 'M',
    );
    Future<
      ({
        ChatNotifier notifier,
        _FakeApiClient api,
        StreamController<StreamEvent> ctrl,
      })
    >
    start(WidgetTester tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);
      unawaited(notifier.sendMessage('go'));
      await tester.pump();
      return (notifier: notifier, api: fakeApi, ctrl: ctrl);
    }

    Future<void> emit(
      WidgetTester tester,
      StreamEvent e,
      StreamController<StreamEvent> ctrl,
    ) async {
      await tester.runAsync(() async {
        ctrl.add(e);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();
    }

    testWidgets('sends exactly one cancel and keeps listening', (tester) async {
      final r = await start(tester);
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      await emit(tester, const TokenEvent('Hello'), r.ctrl);

      r.notifier.stopTurn();
      await tester.pump();
      r.notifier.stopTurn(); // second Stop is ignored
      await tester.pump();

      expect(r.api.cancelTurnCalls, hasLength(1));
      expect(r.api.cancelTurnCalls.single.runId, 'run-1');
      expect(r.api.cancelTurnCalls.single.conversationId, 'conv-1');

      // Still listening: tokens keep rendering while stopping.
      await emit(tester, const TokenEvent(' world'), r.ctrl);
      final s = r.notifier.state.value!;
      expect(s.isStopping, isTrue);
      expect(s.isSending, isTrue);
      expect(s.streamingContent, 'Hello world');

      // The cancelled frame ends the turn (also closes the stall timer).
      await emit(tester, cancelledFrame, r.ctrl);
      expect(r.notifier.state.value!.isStopping, isFalse);
    });

    testWidgets('done after Stop finalizes the message as ok', (tester) async {
      final r = await start(tester);
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await emit(
        tester,
        const DoneEvent(
          role: 'assistant',
          content: 'Full answer',
          messageId: 'M',
        ),
        r.ctrl,
      );

      final s = r.notifier.state.value!;
      expect(s.isStopping, isFalse);
      expect(s.isSending, isFalse);
      final assistant = s.messages.where((m) => !m.isUser).single;
      expect(assistant.status, MessageStatus.ok);
      expect(assistant.content, 'Full answer');
    });

    testWidgets('cancelled frame clears isStopping', (tester) async {
      final r = await start(tester);
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await emit(
        tester,
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'Turn cancelled',
          partialContent: 'part',
          messageId: 'M',
        ),
        r.ctrl,
      );

      final s = r.notifier.state.value!;
      expect(s.isStopping, isFalse);
      expect(
        s.messages.where((m) => !m.isUser).single.status,
        MessageStatus.stopped,
      );
    });

    testWidgets('with nothing in flight sends nothing', (tester) async {
      final fakeApi = _FakeApiClient();
      final notifier = await _pumpTestApp(tester, fakeApi);

      notifier.stopTurn();
      await tester.pump();

      expect(fakeApi.cancelTurnCalls, isEmpty);
      expect(notifier.state.value!.isStopping, isFalse);
    });

    testWidgets('before RunStartedEvent sends the cancel when it arrives', (
      tester,
    ) async {
      final r = await start(tester);

      r.notifier.stopTurn();
      await tester.pump();
      expect(r.api.cancelTurnCalls, isEmpty);
      expect(r.notifier.state.value!.isStopping, isTrue);

      await emit(tester, const RunStartedEvent('run-late'), r.ctrl);

      expect(r.api.cancelTurnCalls.map((c) => c.runId), ['run-late']);
      await emit(tester, cancelledFrame, r.ctrl);
    });

    testWidgets('before a run id then ErrorEvent fails the message quietly', (
      tester,
    ) async {
      final r = await start(tester);
      r.notifier.stopTurn();
      await tester.pump();
      await emit(tester, const ErrorEvent('boom'), r.ctrl);

      final s = r.notifier.state.value!;
      expect(s.messages.single.status, MessageStatus.failed);
      expect(s.error, isNull);
      expect(s.isSending, isFalse);
      expect(s.isStopping, isFalse);
      expect(r.api.cancelTurnCalls, isEmpty);
    });
  });

  group('stopTurn — bounded wait', () {
    const stoppedMessage =
        "Couldn't reach the server to stop this reply. It may still finish.";
    const unconfirmed =
        "The server hasn't confirmed the stop. The reply may still finish.";
    const noRunId = "Couldn't confirm the stop. The reply may still finish.";
    const cancelledFrame = AgentErrorEvent(
      reason: AgentErrorReason.cancelled,
      message: 'Turn cancelled',
      partialContent: 'part',
      messageId: 'M',
    );

    Future<
      ({
        ChatNotifier notifier,
        _FakeApiClient api,
        StreamController<StreamEvent> ctrl,
      })
    >
    start(WidgetTester tester, {int queued = 0}) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);
      unawaited(notifier.sendMessage('go'));
      await tester.pump();
      for (var i = 0; i < queued; i++) {
        await notifier.sendMessage('queued-$i');
      }
      return (notifier: notifier, api: fakeApi, ctrl: ctrl);
    }

    Future<void> emit(
      WidgetTester tester,
      StreamEvent e,
      StreamController<StreamEvent> ctrl,
    ) async {
      await tester.runAsync(() async {
        ctrl.add(e);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      // The queue drain resumes a few microtask hops after the stream ends.
      await tester.pump();
      await tester.pump();
    }

    /// Lets a queued turn that started after a detach finish, so no stall
    /// timer outlives the test.
    Future<void> finishNext(
      WidgetTester tester,
      StreamController<StreamEvent> next,
    ) async {
      await emit(
        tester,
        const DoneEvent(role: 'assistant', content: 'ok', messageId: 'N'),
        next,
      );
    }

    testWidgets('cancel null twice then running: no error, keeps listening', (
      tester,
    ) async {
      final r = await start(tester);
      r.api.cancelResults.addAll([null, null, TurnState.running]);
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      await tester.pump(const Duration(seconds: 2));

      expect(r.api.cancelTurnCalls, hasLength(3));
      var s = r.notifier.state.value!;
      expect(s.error, isNull);
      expect(s.isStopping, isTrue);

      await emit(tester, cancelledFrame, r.ctrl);
      s = r.notifier.state.value!;
      expect(s.isStopping, isFalse);
      expect(s.error, isNull);
    });

    testWidgets('cancel null three times detaches and resumes the queue', (
      tester,
    ) async {
      final r = await start(tester, queued: 1);
      final next = r.api.enqueueStream();
      r.api.cancelResults.addAll([null, null, null]);
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      await emit(tester, const TokenEvent('Hello'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      await tester.pump(const Duration(seconds: 2));
      await tester.pump();

      final s = r.notifier.state.value!;
      expect(r.api.cancelTurnCalls, hasLength(3));
      expect(s.error, stoppedMessage);
      expect(s.isStopping, isFalse);
      final stopped = s.messages.firstWhere(
        (m) => !m.isUser && m.status == MessageStatus.stopped,
      );
      expect(stopped.content, 'Hello');
      expect(
        s.messages.firstWhere((m) => m.content == 'go').status,
        MessageStatus.ok,
      );
      // The queue resumed without waiting for another server event.
      expect(r.api.sentMessages, ['go', 'queued-0']);
      expect(s.pendingQueue, isEmpty);
      await finishNext(tester, next);
    });

    testWidgets(
      'cancel unknown detaches and refreshes; the queue is kept and a late '
      'refresh does not overwrite the next turn',
      (tester) async {
        final r = await start(tester, queued: 2);
        final next = r.api.enqueueStream();
        r.api.cancelResults.add(TurnState.unknown);
        r.api.conversationsFake.gate = Completer<void>();
        r.api.conversationsFake.serverMessages = [
          ('u1', 'user', 'go'),
          ('a1', 'assistant', 'server reply'),
        ];
        await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
        r.notifier.stopTurn();
        await tester.pump();
        await tester.pump();

        expect(r.api.conversationsFake.getConversationCalls, 1);
        // The next queued turn already started; one message is still queued.
        var s = r.notifier.state.value!;
        expect(r.api.sentMessages, ['go', 'queued-0']);
        expect(s.pendingQueue.map((p) => p.text), ['queued-1']);

        // The refresh completes late and must not clobber that turn.
        r.api.conversationsFake.gate!.complete();
        await tester.pump();
        await tester.pump();
        s = r.notifier.state.value!;
        expect(s.isSending, isTrue);
        expect(s.pendingQueue.map((p) => p.text), ['queued-1']);
        expect(s.messages.any((m) => m.content == 'queued-0'), isTrue);
        expect(s.messages.any((m) => m.content == 'server reply'), isFalse);

        final third = r.api.enqueueStream();
        await finishNext(tester, next);
        await finishNext(tester, third);
      },
    );

    testWidgets('cancel unknown without a next turn shows the server state', (
      tester,
    ) async {
      final r = await start(tester, queued: 0);
      r.api.cancelResults.add(TurnState.unknown);
      r.api.conversationsFake.serverMessages = [
        ('u1', 'user', 'go'),
        ('a1', 'assistant', 'server reply'),
      ];
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump();
      await tester.pump();

      final s = r.notifier.state.value!;
      expect(s.isSending, isFalse);
      expect(s.isStopping, isFalse);
      expect(s.messages.map((m) => m.content), ['go', 'server reply']);
    });

    testWidgets(
      'the stall-recovery refresh maps a cancelled reply to stopped',
      (tester) async {
        final r = await start(tester, queued: 0);
        r.api.cancelResults.add(TurnState.unknown);
        r.api.conversationsFake.serverMessages = [
          ('u1', 'user', 'go'),
          ('a1', 'assistant', 'The capital'),
        ];
        r.api.conversationsFake.finishReasons['a1'] = 'cancelled';
        await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
        r.notifier.stopTurn();
        await tester.pump();
        await tester.pump();
        await tester.pump();

        final reply = r.notifier.state.value!.messages.last;
        expect(reply.content, 'The capital');
        expect(reply.status, MessageStatus.stopped);
      },
    );

    testWidgets('grace expiry with a running probe detaches with the '
        'unconfirmed-stop error', (tester) async {
      final r = await start(tester, queued: 1);
      final next = r.api.enqueueStream();
      r.api.nextTurnStatus = TurnState.running;
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      await emit(tester, const TokenEvent('Hello'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump(const Duration(seconds: 9));
      expect(r.notifier.state.value!.isStopping, isTrue);
      expect(r.api.turnStatusCalls, isEmpty);

      await tester.pump(const Duration(seconds: 1));
      await tester.pump();

      expect(r.api.turnStatusCalls.map((c) => c.runId), ['run-1']);
      final s = r.notifier.state.value!;
      expect(s.error, unconfirmed);
      expect(s.isStopping, isFalse);
      expect(
        s.messages
            .firstWhere((m) => !m.isUser && m.status == MessageStatus.stopped)
            .content,
        'Hello',
      );
      expect(r.api.sentMessages, ['go', 'queued-0']);
      await finishNext(tester, next);
    });

    testWidgets('grace expiry with an awaiting-input probe detaches with the '
        'unconfirmed-stop error', (tester) async {
      final r = await start(tester, queued: 1);
      final next = r.api.enqueueStream();
      r.api.nextTurnStatus = TurnState.awaitingInput;
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      await emit(tester, const TokenEvent('Hello'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump(ChatStreamSession.stopGrace);
      await tester.pump();

      final s = r.notifier.state.value!;
      expect(s.error, unconfirmed);
      expect(s.isStopping, isFalse);
      await finishNext(tester, next);
    });

    testWidgets('grace expiry with a completed probe shows the server answer', (
      tester,
    ) async {
      final r = await start(tester);
      r.api.nextTurnStatus = TurnState.completed;
      r.api.conversationsFake.serverMessages = [
        ('u1', 'user', 'go'),
        ('a1', 'assistant', 'Full answer'),
      ];
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      await emit(tester, const TokenEvent('Hel'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump(ChatStreamSession.stopGrace);
      await tester.pump();
      await tester.pump();

      final s = r.notifier.state.value!;
      expect(s.isStopping, isFalse);
      expect(s.error, isNull);
      expect(s.messages.map((m) => m.content), ['go', 'Full answer']);
      expect(s.messages.any((m) => m.status == MessageStatus.stopped), isFalse);
    });

    testWidgets('Stop before a run id detaches after the grace period', (
      tester,
    ) async {
      final r = await start(tester, queued: 1);
      final next = r.api.enqueueStream();
      r.notifier.stopTurn();
      await tester.pump();
      await tester.pump(const Duration(seconds: 9));
      expect(r.notifier.state.value!.isStopping, isTrue);

      await tester.pump(const Duration(seconds: 1));
      await tester.pump();

      final s = r.notifier.state.value!;
      expect(r.api.cancelTurnCalls, isEmpty);
      expect(r.api.turnStatusCalls, isEmpty);
      expect(s.error, noRunId);
      expect(s.isStopping, isFalse);
      expect(
        s.messages.firstWhere((m) => m.content == 'go').status,
        MessageStatus.ok,
      );
      expect(r.api.sentMessages, ['go', 'queued-0']);
      await finishNext(tester, next);
    });

    testWidgets('the first queued message is sent right after the cancelled '
        'frame', (tester) async {
      final r = await start(tester, queued: 2);
      final next = r.api.enqueueStream();
      await emit(tester, const RunStartedEvent('run-1'), r.ctrl);
      r.notifier.stopTurn();
      await tester.pump();
      await emit(tester, cancelledFrame, r.ctrl);

      expect(r.api.sentMessages, ['go', 'queued-0']);
      expect(r.notifier.state.value!.pendingQueue.map((p) => p.text), [
        'queued-1',
      ]);
      final third = r.api.enqueueStream();
      await finishNext(tester, next);
      await finishNext(tester, third);
    });
  });

  group('stopTurn — voice run id', () {
    testWidgets(
      'Stop before the voice run id never cancels the previous turn',
      (tester) async {
        final fakeApi = _FakeApiClient();
        final text = fakeApi.enqueueStream();
        final voice = fakeApi.enqueueVoiceStream();
        final notifier = await _pumpTestApp(tester, fakeApi);

        unawaited(notifier.sendMessage('go'));
        await tester.pump();
        await tester.runAsync(() async {
          text.add(const RunStartedEvent('T'));
          text.add(
            const DoneEvent(role: 'assistant', content: 'a', messageId: 'M'),
          );
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pump();

        unawaited(
          notifier.sendVoiceMessage(Uint8List.fromList([1, 2]), 'audio/webm'),
        );
        await tester.pump();
        notifier.stopTurn();
        await tester.pump();
        expect(fakeApi.cancelTurnCalls, isEmpty);

        await tester.runAsync(() async {
          voice.add(const RunStartedEvent('V'));
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pump();

        expect(fakeApi.cancelTurnCalls.map((c) => c.runId), ['V']);
        await tester.runAsync(() async {
          voice.add(
            const AgentErrorEvent(
              reason: AgentErrorReason.cancelled,
              message: 'Turn cancelled',
              partialContent: '',
              userMessagePersisted: false,
            ),
          );
          await Future<void>.delayed(Duration.zero);
          await Future<void>.delayed(Duration.zero);
        });
        await tester.pump();
      },
    );
  });

  group('ChatNotifier.lifecycle', () {
    Future<void> deliver(
      WidgetTester tester,
      StreamController<StreamEvent> ctrl,
      StreamEvent e,
    ) async {
      await tester.runAsync(() async {
        ctrl.add(e);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();
    }

    Future<
      ({
        ChatNotifier notifier,
        _FakeApiClient api,
        StreamController<StreamEvent> ctrl,
        List<ChatLifecycleEvent> events,
      })
    >
    start(WidgetTester tester) async {
      final fakeApi = _FakeApiClient();
      final ctrl = fakeApi.enqueueStream();
      final notifier = await _pumpTestApp(tester, fakeApi);
      final events = <ChatLifecycleEvent>[];
      notifier.lifecycle.listen(events.add);
      unawaited(notifier.sendMessage('go'));
      await tester.pump();
      return (notifier: notifier, api: fakeApi, ctrl: ctrl, events: events);
    }

    testWidgets('a live turn emits TurnStarted then TurnCompleted(live)', (
      tester,
    ) async {
      final r = await start(tester);
      expect(r.events.map((e) => e.runtimeType), [TurnStarted]);

      await deliver(tester, r.ctrl, const RunStartedEvent('run-1'));
      await deliver(tester, r.ctrl, const TokenEvent('**Hi**'));
      await deliver(
        tester,
        r.ctrl,
        const DoneEvent(role: 'assistant', content: '**Hi**'),
      );
      await tester.pumpAndSettle();

      expect(r.events.map((e) => e.runtimeType), [TurnStarted, TurnCompleted]);
      final done = r.events.last as TurnCompleted;
      expect(done.source, TurnSource.live);
      expect(done.content, '**Hi**');
      unawaited(r.ctrl.close());
    });

    testWidgets('a resumed replay emits one TurnCompleted(replay)', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      final ctrl1 = fakeApi.enqueueStream();
      fakeApi.enqueueReplayStream()
        ..add(const TokenEvent('hello'))
        ..add(const DoneEvent(role: 'assistant', content: 'hello'))
        ..close();
      final notifier = await _pumpTestApp(tester, fakeApi);
      final events = <ChatLifecycleEvent>[];
      notifier.lifecycle.listen(events.add);

      unawaited(notifier.sendMessage('ping'));
      await tester.pump();
      await tester.runAsync(() async {
        ctrl1.add(const RunStartedEvent('run-abc'));
        await Future<void>.delayed(Duration.zero);
        ctrl1.addError(const SocketException('network drop'));
        await ctrl1.close();
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump(const Duration(seconds: 2));
      await tester.pumpAndSettle();

      final completed = events.whereType<TurnCompleted>().toList();
      expect(completed, hasLength(1));
      expect(completed.single.source, TurnSource.replay);
      expect(completed.single.content, 'hello');
      expect(events.whereType<TurnStarted>(), hasLength(1));
    });

    testWidgets('opening a conversation with finished messages is silent', (
      tester,
    ) async {
      final fakeApi = _FakeApiClient();
      fakeApi.conversationsFake.serverMessages = [
        for (var i = 0; i < 10; i++)
          ('m$i', i.isEven ? 'user' : 'assistant', 'message $i'),
      ];
      final notifier = await _pumpTestApp(tester, fakeApi);
      final events = <ChatLifecycleEvent>[];
      notifier.lifecycle.listen(events.add);

      await notifier.loadConversation('conv-1');
      await tester.pump();

      expect(notifier.state.value!.messages, hasLength(10));
      expect(events, isEmpty);
    });

    testWidgets('stopTurn reconciling to stopped emits TurnStopped', (
      tester,
    ) async {
      final r = await start(tester);
      await deliver(tester, r.ctrl, const RunStartedEvent('run-1'));
      await deliver(tester, r.ctrl, const TokenEvent('part'));
      r.notifier.stopTurn();
      await tester.pump();
      expect(r.events.whereType<TurnStopped>(), isEmpty);

      await deliver(
        tester,
        r.ctrl,
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'Turn cancelled',
          partialContent: 'part',
          messageId: 'M',
        ),
      );
      await tester.pumpAndSettle();

      expect(r.events.whereType<TurnStopped>(), hasLength(1));
      expect(r.events.whereType<TurnCompleted>(), isEmpty);
      unawaited(r.ctrl.close());
    });

    testWidgets('_failTurn emits TurnFailed with the message', (tester) async {
      final r = await start(tester);
      await deliver(tester, r.ctrl, const RunStartedEvent('run-1'));
      await deliver(tester, r.ctrl, const ErrorEvent('rate limited'));
      await tester.pumpAndSettle();

      final failed = r.events.whereType<TurnFailed>().toList();
      expect(failed, hasLength(1));
      expect(failed.single.message, 'rate limited');
      unawaited(r.ctrl.close());
    });

    testWidgets('no status update for kTurnStallThreshold emits one stall', (
      tester,
    ) async {
      final r = await start(tester);
      await deliver(tester, r.ctrl, const RunStartedEvent('run-1'));
      await deliver(tester, r.ctrl, const TokenEvent('x'));

      await tester.pump(kTurnStallThreshold - const Duration(seconds: 1));
      expect(r.events.whereType<TurnStalled>(), isEmpty);

      // A status update re-arms the timer.
      await deliver(tester, r.ctrl, const StatusEvent('searching'));
      await tester.pump(kTurnStallThreshold - const Duration(seconds: 1));
      expect(r.events.whereType<TurnStalled>(), isEmpty);

      await tester.pump(const Duration(seconds: 2));
      expect(r.events.whereType<TurnStalled>(), hasLength(1));

      await deliver(
        tester,
        r.ctrl,
        const DoneEvent(role: 'assistant', content: 'x'),
      );
      await tester.pumpAndSettle();
      await tester.pump(kTurnStallThreshold * 2);
      expect(r.events.whereType<TurnStalled>(), hasLength(1));
      unawaited(r.ctrl.close());
    });
  });
}
