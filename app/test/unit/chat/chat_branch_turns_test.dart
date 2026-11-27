// ChatNotifier's branching turns: editing a sent message, regenerating a
// reply, retrying a failed one and taking over while a turn waits for input.

import 'dart:async';
import 'dart:io';

import 'package:assistant_api/assistant_api.dart'
    as api
    hide ServerCapabilities;
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fake server -------------------------------------------------------------

api.BranchInfo _branch(int index, int count, List<String> siblings) =>
    api.BranchInfo(
      (b) => b
        ..index = index
        ..count = count
        ..siblingIds = ListBuilder(siblings),
    );

api.MessageSummary _msg(
  String id,
  String role,
  String content, {
  api.BranchInfo? branch,
  api.BranchInfo? replyBranch,
  List<api.AttachmentMetaResponse>? attachments,
}) => api.MessageSummary(
  (m) => m
    ..id = id
    ..role = role
    ..content = content
    ..createdAt = DateTime(2024)
    ..turn = 0
    ..ttsAvailable = false
    ..branch = branch?.toBuilder()
    ..replyBranch = replyBranch?.toBuilder()
    ..attachments = attachments == null ? null : ListBuilder(attachments),
);

api.AttachmentMetaResponse _attachment(String id) => api.AttachmentMetaResponse(
  (a) => a
    ..id = id
    ..filename = '$id.png'
    ..mimeType = 'image/png'
    ..sizeBytes = 3
    ..url = 'https://example.test/$id.png'
    ..createdAt = DateTime(2024),
);

api.ConversationDetail _detail(List<api.MessageSummary> rows) =>
    api.ConversationDetail(
      (b) => b
        ..id = 'conv-1'
        ..title = 'T'
        ..createdAt = DateTime(2024)
        ..updatedAt = DateTime(2024)
        ..messages.replace(rows),
    );

class _FakeConversationsApi extends api.ConversationsApi {
  _FakeConversationsApi() : super(Dio(), api.standardSerializers);

  /// Rows the next `GET /api/conversations/{id}` reports.
  List<api.MessageSummary> history = [];

  /// Rows the next `switch-branch` reports.
  List<api.MessageSummary> switched = [];

  /// Message ids sent to switch-branch, in call order.
  final List<String> switchedIds = [];

  int getConversationCalls = 0;

  @override
  Future<Response<api.ConversationDetail>> getConversation({
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
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id'),
      statusCode: 200,
      data: _detail(history),
    );
  }

  @override
  Future<Response<api.ConversationDetail>> switchConversationBranch({
    required String id,
    required api.SwitchBranchRequest switchBranchRequest,
    String? xSpaceId,
    CancelToken? cancelToken,
    Map<String, dynamic>? headers,
    Map<String, dynamic>? extra,
    ValidateStatus? validateStatus,
    ProgressCallback? onSendProgress,
    ProgressCallback? onReceiveProgress,
  }) async {
    switchedIds.add(switchBranchRequest.messageId);
    return Response(
      requestOptions: RequestOptions(path: '/api/conversations/$id/switch'),
      statusCode: 200,
      data: _detail(switched),
    );
  }
}

typedef _EditCall = ({String conversationId, String messageId, String text});
typedef _RegenerateCall = ({
  String conversationId,
  String messageId,
  String? model,
});

class _FakeApi extends ApiClient {
  _FakeApi() : super(baseUrl: 'http://fake', token: 'fake');

  final _FakeConversationsApi _conversationsFake = _FakeConversationsApi();

  @override
  api.ConversationsApi get conversations => _conversationsFake;

  _FakeConversationsApi get conversationsFake => _conversationsFake;

  final _sends = <StreamController<StreamEvent>>[];
  final _edits = <StreamController<StreamEvent>>[];
  final _regenerates = <StreamController<StreamEvent>>[];
  final _replays = <Object>[];

  final List<String> sent = [];
  final List<_EditCall> edits = [];
  final List<_RegenerateCall> regenerates = [];

  StreamController<StreamEvent> enqueueSend() {
    final c = StreamController<StreamEvent>();
    _sends.add(c);
    return c;
  }

  StreamController<StreamEvent> enqueueEdit() {
    final c = StreamController<StreamEvent>();
    _edits.add(c);
    return c;
  }

  StreamController<StreamEvent> enqueueRegenerate() {
    final c = StreamController<StreamEvent>();
    _regenerates.add(c);
    return c;
  }

  StreamController<StreamEvent> enqueueReplay() {
    final c = StreamController<StreamEvent>();
    _replays.add(c);
    return c;
  }

  void enqueueReplayError(int status) {
    _replays.add(
      DioException(
        requestOptions: RequestOptions(path: ''),
        response: Response<void>(
          requestOptions: RequestOptions(path: ''),
          statusCode: status,
        ),
        type: DioExceptionType.badResponse,
      ),
    );
  }

  Stream<StreamEvent> _take(List<StreamController<StreamEvent>> queue) =>
      queue.isEmpty ? const Stream.empty() : queue.removeAt(0).stream;

  @override
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
  }) {
    sent.add(message);
    return _take(_sends);
  }

  @override
  Stream<StreamEvent> streamEdit(
    String conversationId,
    String messageId,
    String text,
  ) {
    edits.add((
      conversationId: conversationId,
      messageId: messageId,
      text: text,
    ));
    return _take(_edits);
  }

  @override
  Stream<StreamEvent> streamRegenerate(
    String conversationId,
    String messageId, {
    String? model,
  }) {
    regenerates.add((
      conversationId: conversationId,
      messageId: messageId,
      model: model,
    ));
    return _take(_regenerates);
  }

  @override
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
    CancelToken? cancelToken,
  }) async* {
    if (_replays.isEmpty) return;
    final item = _replays.removeAt(0);
    if (item is DioException) throw item;
    if (item is StreamController<StreamEvent>) yield* item.stream;
  }
}

// -- Fixtures ----------------------------------------------------------------

Future<_Fixture> _setup({List<api.MessageSummary>? history}) async {
  final api = _FakeApi();
  api.conversationsFake.history =
      history ??
      [
        _msg('u1', 'user', 'hello'),
        _msg('a1', 'assistant', 'hi'),
        _msg('u2', 'user', 'original'),
        _msg('a2', 'assistant', 'reply'),
      ];
  final container = ProviderContainer(
    overrides: [
      apiClientProvider.overrideWithValue(api),
      conversationListProvider.overrideWith(_FakeListNotifier.new),
    ],
  );
  addTearDown(container.dispose);
  container.listen(chatProvider, (_, _) {});
  await container.read(chatProvider.future);
  final notifier = container.read(chatProvider.notifier);
  await notifier.loadConversation('conv-1');
  return _Fixture(container: container, api: api, notifier: notifier);
}

class _Fixture {
  _Fixture({
    required this.container,
    required this.api,
    required this.notifier,
  });

  final ProviderContainer container;
  final _FakeApi api;
  final ChatNotifier notifier;

  ChatState get state => container.read(chatProvider).value!;

  List<String> get ids => state.messages.map((m) => m.id).toList();

  ChatMessage message(String id) =>
      state.messages.firstWhere((m) => m.id == id);
}

class _FakeListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();
}

Future<void> _flush() async {
  for (var i = 0; i < 5; i++) {
    await Future<void>.delayed(Duration.zero);
  }
}

InteractionRequestedEvent _approval() => InteractionRequestedEvent(
  interactionId: 'int-1',
  kind: 'tool_approval',
  createdAt: DateTime.utc(2026),
  expiresAt: DateTime.utc(2026, 1, 1, 0, 10),
  request: const {
    'tool_name': 'bash',
    'arguments': {'command': 'git status'},
    'options': ['allow_once', 'reject_once'],
  },
);

InteractionResolvedEvent _resolvedCancelled() => InteractionResolvedEvent(
  interactionId: 'int-1',
  kind: 'tool_approval',
  outcome: InteractionOutcome.cancelled,
  resolvedAt: DateTime.utc(2026, 1, 1, 0, 1),
);

/// Put the notifier into the "turn waits for an approval" state: a live send
/// whose stream stays open after the interaction request.
Future<StreamController<StreamEvent>> _awaitInput(
  _Fixture f, {
  String message = 'hello again',
}) async {
  final send = f.api.enqueueSend();
  await f.notifier.sendMessage(message);
  await _flush();
  send.add(const RunStartedEvent('run-old'));
  send.add(_approval());
  await _flush();
  expect(f.state.currentTurnStatus?.awaitingLabel, isNotNull);
  return send;
}

void main() {
  group('ChatNotifier.editMessage', () {
    test('truncates after the message and keeps its attachments', () async {
      final f = await _setup(
        history: [
          _msg('u1', 'user', 'hello'),
          _msg('a1', 'assistant', 'hi'),
          _msg('u2', 'user', 'original', attachments: [_attachment('att-1')]),
          _msg('a2', 'assistant', 'reply'),
        ],
      );
      final edit = f.api.enqueueEdit();

      unawaited(f.notifier.editMessage('u2', 'edited text'));
      await _flush();

      expect(f.api.edits, hasLength(1));
      expect(f.api.edits.single.conversationId, 'conv-1');
      expect(f.api.edits.single.messageId, 'u2');
      expect(f.api.edits.single.text, 'edited text');

      final msgs = f.state.messages;
      expect(msgs.sublist(0, 2).map((m) => m.id), ['u1', 'a1']);
      final bubble = msgs[2];
      expect(bubble.role, 'user');
      expect(bubble.content, 'edited text');
      expect(bubble.status, MessageStatus.sending);
      expect(bubble.attachments.map((a) => a.id), ['att-1']);
      expect(msgs.last.id, streamingPlaceholderId);
      expect(f.state.isSending, isTrue);
      await edit.close();
    });

    test(
      'replaces the optimistic id with run_started user_message_id',
      () async {
        final f = await _setup();
        final edit = f.api.enqueueEdit();

        unawaited(f.notifier.editMessage('u2', 'edited text'));
        await _flush();
        final optimisticId = f.state.messages[2].id;

        edit.add(const RunStartedEvent('run-1', userMessageId: 'u2-prime'));
        await _flush();

        expect(f.state.messages[2].id, 'u2-prime');
        expect(
          f.state.messages.map((m) => m.id),
          isNot(contains(optimisticId)),
        );
        await edit.close();
      },
    );

    test('reloads the detail on done', () async {
      final f = await _setup();
      final edit = f.api.enqueueEdit();
      unawaited(f.notifier.editMessage('u2', 'edited text'));
      await _flush();

      f.api.conversationsFake.history = [
        _msg('u1', 'user', 'hello'),
        _msg('a1', 'assistant', 'hi'),
        _msg(
          'u2-prime',
          'user',
          'edited text',
          branch: _branch(1, 2, ['u2', 'u2-prime']),
        ),
        _msg('a2-prime', 'assistant', 'new reply'),
      ];
      edit.add(const RunStartedEvent('run-1', userMessageId: 'u2-prime'));
      edit.add(
        const DoneEvent(
          role: 'assistant',
          content: 'new reply',
          messageId: 'a2-prime',
        ),
      );
      await edit.close();
      await _flush();

      expect(f.ids, ['u1', 'a1', 'u2-prime', 'a2-prime']);
      expect(f.message('u2-prime').branch?.count, 2);
      expect(f.state.isSending, isFalse);
    });

    test('marks the bubble failed on agent_error', () async {
      final f = await _setup();
      final edit = f.api.enqueueEdit();
      unawaited(f.notifier.editMessage('u2', 'edited text'));
      await _flush();

      edit.add(const RunStartedEvent('run-1', userMessageId: 'u2-prime'));
      edit.add(
        const AgentErrorEvent(reason: AgentErrorReason.error, message: 'boom'),
      );
      await _flush();

      expect(f.message('u2-prime').status, MessageStatus.failed);
      expect(f.state.error, 'boom');
      expect(f.state.isSending, isFalse);
    });

    test('refuses while a turn works', () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('busy');
      await _flush();
      final before = f.state.messages;

      await f.notifier.editMessage('u2', 'edited text');
      await _flush();

      expect(f.api.edits, isEmpty);
      expect(f.state.messages, same(before));
      await send.close();
    });
  });

  group('ChatNotifier.regenerate', () {
    test('truncates from the reply and reloads the new branch', () async {
      final f = await _setup();
      final regenerate = f.api.enqueueRegenerate();

      unawaited(f.notifier.regenerate('a2'));
      await _flush();

      expect(f.api.regenerates.single.messageId, 'a2');
      expect(f.ids, ['u1', 'a1', 'u2', streamingPlaceholderId]);

      f.api.conversationsFake.history = [
        _msg('u1', 'user', 'hello'),
        _msg('a1', 'assistant', 'hi'),
        _msg('u2', 'user', 'original'),
        _msg('a2-prime', 'assistant', 'second reply'),
      ];
      regenerate.add(const RunStartedEvent('run-1', userMessageId: 'u2'));
      regenerate.add(
        const DoneEvent(
          role: 'assistant',
          content: 'second reply',
          messageId: 'a2-prime',
        ),
      );
      await regenerate.close();
      await _flush();

      expect(f.ids, ['u1', 'a1', 'u2', 'a2-prime']);
      expect(f.state.isSending, isFalse);
    });

    test('a cancelled regenerate reloads the old reply', () async {
      final f = await _setup();
      final regenerate = f.api.enqueueRegenerate();
      unawaited(f.notifier.regenerate('a2'));
      await _flush();

      regenerate.add(const RunStartedEvent('run-1', userMessageId: 'u2'));
      regenerate.add(
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'cancelled',
        ),
      );
      await _flush();

      expect(f.ids, ['u1', 'a1', 'u2', 'a2']);
      expect(f.state.error, isNull);
      expect(f.message('u2').status, MessageStatus.ok);
    });

    test('refuses while a turn works', () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('busy');
      await _flush();
      final before = f.state.messages;

      await f.notifier.regenerate('a2');
      await _flush();

      expect(f.api.regenerates, isEmpty);
      expect(f.state.messages, same(before));
      await send.close();
    });
  });

  group('branch turns while the turn waits for input', () {
    test('an edit is accepted and the old stream cannot fail it', () async {
      final f = await _setup();
      final old = await _awaitInput(f);
      final edit = f.api.enqueueEdit();

      unawaited(f.notifier.editMessage('u1', 'fixed'));
      await _flush();
      expect(f.api.edits.single.messageId, 'u1');

      old.add(_resolvedCancelled());
      old.add(
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'cancelled',
        ),
      );
      await _flush();

      expect(f.state.error, isNull);
      expect(
        f.state.messages.where((m) => m.status == MessageStatus.failed),
        isEmpty,
      );

      f.api.conversationsFake.history = [
        _msg('u1-prime', 'user', 'fixed'),
        _msg('a1-prime', 'assistant', 'new answer'),
      ];
      edit.add(const RunStartedEvent('run-1', userMessageId: 'u1-prime'));
      edit.add(
        const DoneEvent(
          role: 'assistant',
          content: 'new answer',
          messageId: 'a1-prime',
        ),
      );
      await edit.close();
      await _flush();

      expect(f.ids, ['u1-prime', 'a1-prime']);
    });

    test('a regenerate is accepted while the turn waits for input', () async {
      final f = await _setup();
      final old = await _awaitInput(f);
      final regenerate = f.api.enqueueRegenerate();

      unawaited(f.notifier.regenerate('a2'));
      await _flush();
      expect(f.api.regenerates.single.messageId, 'a2');
      expect(f.ids.last, streamingPlaceholderId);

      old.add(_resolvedCancelled());
      old.add(
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'cancelled',
        ),
      );
      await _flush();
      expect(f.state.error, isNull);

      f.api.conversationsFake.history = [
        _msg('u1', 'user', 'hello'),
        _msg('a1', 'assistant', 'hi'),
        _msg('u2', 'user', 'original'),
        _msg('a2-prime', 'assistant', 'second reply'),
      ];
      regenerate.add(const RunStartedEvent('run-1', userMessageId: 'u2'));
      regenerate.add(
        const DoneEvent(
          role: 'assistant',
          content: 'second reply',
          messageId: 'a2-prime',
        ),
      );
      await regenerate.close();
      await _flush();

      expect(f.ids, ['u1', 'a1', 'u2', 'a2-prime']);
    });

    test('a switch is accepted and the old stream cannot rewrite it', () async {
      final f = await _setup();
      final old = await _awaitInput(f);

      f.api.conversationsFake.switched = [
        _msg('u1-b', 'user', 'other'),
        _msg('a1-b', 'assistant', 'other answer'),
      ];
      await f.notifier.switchBranch('u1-b');
      await _flush();

      old.add(_resolvedCancelled());
      old.add(
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'cancelled',
          messageId: 'a1-stopped',
        ),
      );
      await _flush();

      expect(f.ids, ['u1-b', 'a1-b']);
      expect(f.state.error, isNull);
      expect(f.state.isSending, isFalse);
    });
  });

  group('retryMessage', () {
    test('a replay ending in agent_error regenerates the message', () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('hello');
      await _flush();
      send.add(const RunStartedEvent('run-1', userMessageId: 'U1'));
      send.add(
        const AgentErrorEvent(reason: AgentErrorReason.error, message: 'boom'),
      );
      await _flush();
      final failed = f.message('U1');
      expect(failed.status, MessageStatus.failed);

      final replay = f.api.enqueueReplay();
      replay.add(const RunStartedEvent('run-1'));
      replay.add(
        const AgentErrorEvent(reason: AgentErrorReason.error, message: 'boom'),
      );
      unawaited(replay.close());
      unawaited(f.api.enqueueRegenerate().close());

      await f.notifier.retryMessage(failed);
      await _flush();

      expect(f.api.regenerates.single.messageId, 'U1');
      expect(f.api.sent, ['hello']);
    });

    test('a replay 404 regenerates the message', () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('hello');
      await _flush();
      send.add(const RunStartedEvent('run-1', userMessageId: 'U1'));
      send.add(
        const AgentErrorEvent(reason: AgentErrorReason.error, message: 'boom'),
      );
      await _flush();
      final failed = f.message('U1');

      f.api.enqueueReplayError(404);
      unawaited(f.api.enqueueRegenerate().close());

      await f.notifier.retryMessage(failed);
      await _flush();

      expect(f.api.regenerates.single.messageId, 'U1');
      expect(f.api.sent, ['hello']);
    });

    test('a regenerate 404 falls back to sending', () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('hello');
      await _flush();
      send.add(const RunStartedEvent('run-1', userMessageId: 'U1'));
      send.add(
        const AgentErrorEvent(reason: AgentErrorReason.error, message: 'boom'),
      );
      await _flush();
      final failed = f.message('U1');

      f.api.enqueueReplayError(404);
      final regenerate = f.api.enqueueRegenerate()
        ..add(const ErrorEvent('not found', statusCode: 404))
        ..close();
      unawaited(regenerate.done);

      await f.notifier.retryMessage(failed);
      await _flush();

      expect(f.api.regenerates.single.messageId, 'U1');
      expect(f.api.sent, ['hello', 'hello']);
    });

    test('without a server id the message is sent again', () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('hello');
      await _flush();
      send.add(const ErrorEvent('offline'));
      await _flush();
      final failed = f.state.messages.lastWhere((m) => m.isUser);
      expect(failed.status, MessageStatus.failed);

      await f.notifier.retryMessage(failed);
      await _flush();

      expect(f.api.regenerates, isEmpty);
      expect(f.api.sent, ['hello', 'hello']);
    });

    test('a failed edit regenerates U-prime and re-edits on 404', () async {
      final f = await _setup();
      final edit = f.api.enqueueEdit();
      unawaited(f.notifier.editMessage('u2', 'edited text'));
      await _flush();
      edit.add(const RunStartedEvent('run-1', userMessageId: 'u2-prime'));
      edit.add(
        const AgentErrorEvent(reason: AgentErrorReason.error, message: 'boom'),
      );
      await _flush();
      final failed = f.message('u2-prime');
      expect(failed.status, MessageStatus.failed);

      final regenerate = f.api.enqueueRegenerate()
        ..add(const ErrorEvent('not found', statusCode: 404))
        ..close();
      unawaited(regenerate.done);
      unawaited(f.api.enqueueEdit().close());

      await f.notifier.retryMessage(failed);
      await _flush();

      expect(f.api.regenerates.single.messageId, 'u2-prime');
      expect(f.api.edits.map((e) => (e.messageId, e.text)), [
        ('u2', 'edited text'),
        ('u2', 'edited text'),
      ]);
      expect(f.api.sent, isEmpty);
    });
  });

  test(
    'a send answered with 409 keeps the failed message and shows it',
    () async {
      final f = await _setup();
      final send = f.api.enqueueSend();
      await f.notifier.sendMessage('hello');
      await _flush();
      send.add(const ErrorEvent('busy', statusCode: 409));
      await _flush();

      final failed = f.state.messages.lastWhere((m) => m.isUser);
      expect(failed.content, 'hello');
      expect(failed.status, MessageStatus.failed);
      expect(f.state.error, 'busy');
      expect(f.state.isSending, isFalse);
    },
  );

  testWidgets('a reconnect during an edit reloads the edited branch', (
    tester,
  ) async {
    final f = await _setup();
    final edit = f.api.enqueueEdit();
    unawaited(f.notifier.editMessage('u2', 'edited text'));
    await tester.pump();

    for (var i = 0; i < 3; i++) {
      f.api.enqueueReplayError(404);
    }

    await tester.runAsync(() async {
      edit.add(const RunStartedEvent('run-1', userMessageId: 'u2-prime'));
      edit.add(const TokenEvent('new'));
      await Future<void>.delayed(Duration.zero);
      edit.addError(const HttpException('Connection closed'));
      await edit.close();
      for (var i = 0; i < 4; i++) {
        await Future<void>.delayed(Duration.zero);
      }
    });
    // Advance through the replay backoff delays (1s, 2s, 4s).
    await tester.pump(const Duration(seconds: 2));
    await tester.pump(const Duration(seconds: 3));
    await tester.pump(const Duration(seconds: 5));
    await tester.pumpAndSettle();
    expect(f.notifier.needsReconnect, isTrue);

    f.api.conversationsFake.history = [
      _msg('u1', 'user', 'hello'),
      _msg('a1', 'assistant', 'hi'),
      _msg(
        'u2-prime',
        'user',
        'edited text',
        branch: _branch(1, 2, ['u2', 'u2-prime']),
      ),
      _msg('a2-prime', 'assistant', 'new reply'),
    ];

    await tester.runAsync(f.notifier.attemptReconnect);
    await tester.pumpAndSettle();

    expect(f.ids, ['u1', 'a1', 'u2-prime', 'a2-prime']);
    expect(f.message('u2-prime').branch?.count, 2);
    expect(f.ids, isNot(contains('a2')));
    expect(f.state.isSending, isFalse);
  });
}
