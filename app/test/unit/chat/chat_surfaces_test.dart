// RED: openspec/changes/generative-ui-a2ui PR 15.3 — the notifier turns live
// `surface_update` events into timeline entries, updates them in place, closes
// active surfaces when the user sends the next message and drops the chip of a
// successful render-surface call.

import 'dart:async';

import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/api/models/surface_action.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:dio/dio.dart' show CancelToken;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _FakeApiClient extends ApiClient {
  _FakeApiClient() : super(baseUrl: 'http://fake', token: 'fake');

  final List<StreamController<StreamEvent>> _queue = [];

  StreamController<StreamEvent> enqueueStream() {
    final controller = StreamController<StreamEvent>();
    _queue.add(controller);
    return controller;
  }

  final List<String> sentMessages = [];

  /// The surface action each `streamMessages` call carried, in call order.
  final List<SurfaceActionSubmission?> sentSurfaceActions = [];

  @override
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
    SurfaceActionSubmission? surfaceAction,
  }) {
    sentMessages.add(message);
    sentSurfaceActions.add(surfaceAction);
    if (_queue.isNotEmpty) return _queue.removeAt(0).stream;
    return const Stream.empty();
  }

  @override
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
    CancelToken? cancelToken,
  }) async* {}
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}

  @override
  void prependConversation(ConversationSummary conv) {}
}

class _TestApp extends ConsumerWidget {
  const _TestApp({required this.onBuild});

  final void Function(ChatNotifier) onBuild;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    onBuild(ref.watch(chatProvider.notifier));
    return const MaterialApp(home: Scaffold());
  }
}

Future<ChatNotifier> _pump(WidgetTester tester, _FakeApiClient api) async {
  late ChatNotifier notifier;
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        apiClientProvider.overrideWithValue(api),
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

const _createMessages = [
  {
    'version': '1.0',
    'createSurface': {
      'surfaceId': 'surface-1',
      'catalogId': 'https://assistant.invalid/a2ui/catalogs/assistant/v1',
      'components': [
        {'id': 'root', 'component': 'Text', 'text': 'Sunny'},
      ],
    },
  },
];

SurfaceUpdateEvent _surface({
  String surfaceId = 'surface-1',
  String status = 'active',
  List<Map<String, Object?>> messages = _createMessages,
  String toolCallId = 'call-1',
}) {
  return SurfaceUpdateEvent(
    surfaceId: surfaceId,
    toolCallId: toolCallId,
    anchorMessageId: 'message-1',
    anchorCallIndex: 0,
    title: 'Weather',
    fallbackText: 'Sunny, 21 °C',
    status: status,
    messages: messages,
  );
}

Future<void> _deliver(
  WidgetTester tester,
  StreamController<StreamEvent> controller,
  List<StreamEvent> events,
) async {
  await tester.runAsync(() async {
    for (final event in events) {
      controller.add(event);
    }
    await Future<void>.delayed(Duration.zero);
  });
  await tester.pumpAndSettle();
}

Future<void> _finishTurn(
  WidgetTester tester,
  StreamController<StreamEvent> controller,
) async {
  await _deliver(tester, controller, [
    const DoneEvent(role: 'assistant', content: 'Done'),
  ]);
  await tester.runAsync(controller.close);
}

ChatMessage? _surfaceEntry(ChatState state) {
  for (final message in state.messages) {
    if (message.timelineType == TimelineEntryType.surface) return message;
  }
  return null;
}

SurfaceActionSubmission _submission() => const SurfaceActionSubmission(
  surfaceId: 'surface-1',
  name: 'submit_form',
  sourceComponentId: 'button-1',
  context: {'city': 'Berlin'},
  dataModel: {'unit': 'c'},
);

void main() {
  testWidgets('a live surface_update adds a surface entry', (tester) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [_surface()]);

    final entry = _surfaceEntry(notifier.state.value!);
    expect(entry, isNotNull, reason: 'the surface becomes a timeline entry');
    expect(entry!.surface!.surfaceId, 'surface-1');
    expect(entry.surface!.toolCallId, 'call-1');
    expect(entry.surface!.title, 'Weather');
    expect(entry.surface!.answerable, isTrue);
    expect(entry.surface!.messages, hasLength(1));

    await _finishTurn(tester, controller);
  });

  testWidgets('a status event updates the surface in place', (tester) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [_surface()]);
    await _deliver(tester, controller, [
      _surface(status: 'submitted', messages: const []),
    ]);

    final entries = notifier.state.value!.messages
        .where((m) => m.timelineType == TimelineEntryType.surface)
        .toList();
    expect(entries, hasLength(1), reason: 'the status event replaces no entry');
    expect(entries.single.surface!.status, SurfaceStatus.submitted);
    expect(
      entries.single.surface!.messages,
      hasLength(1),
      reason: 'an empty status frame keeps the rendered messages',
    );

    await _finishTurn(tester, controller);
  });

  testWidgets('sending the next message closes every active surface', (
    tester,
  ) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [_surface()]);
    await _finishTurn(tester, controller);

    final next = api.enqueueStream();
    unawaited(notifier.sendMessage('thanks'));
    await tester.pump();

    final entry = _surfaceEntry(notifier.state.value!);
    expect(entry, isNotNull, reason: 'closing keeps the surface in place');
    expect(entry!.surface!.status, SurfaceStatus.active);
    expect(entry.surface!.answerable, isFalse);
    expect(entry.surface!.isClosed, isTrue);

    await _finishTurn(tester, next);
  });

  testWidgets('a queued message closes surfaces only when it is sent', (
    tester,
  ) async {
    final api = _FakeApiClient();
    final first = api.enqueueStream();
    final second = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, first, [_surface()]);

    unawaited(notifier.sendMessage('one more'));
    await tester.pump();
    expect(
      _surfaceEntry(notifier.state.value!)!.surface!.answerable,
      isTrue,
      reason: 'a queued message has not closed the surface yet',
    );

    await _finishTurn(tester, first);
    await _deliver(tester, second, [const TokenEvent('hi')]);

    final entry = _surfaceEntry(notifier.state.value!);
    expect(entry!.surface!.answerable, isFalse);

    await _finishTurn(tester, second);
  });

  testWidgets('an ok render-surface result renders no chip', (tester) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [
      const StatusEvent('Calling tool: render-surface', toolCallId: 'call-1'),
      _surface(),
      ToolResultEvent.fromJson({
        'tool_name': 'render-surface',
        'status': 'ok',
        'tool_call_id': 'call-1',
        'result': 'rendered',
      }),
    ]);

    final state = notifier.state.value!;
    expect(_surfaceEntry(state), isNotNull);
    expect(
      state.messages.where(
        (m) =>
            m.timelineType == TimelineEntryType.toolCall &&
            m.toolCalls.first.toolName == 'render-surface',
      ),
      isEmpty,
      reason: 'the surface is the visible result, not a chip',
    );

    await _finishTurn(tester, controller);
  });

  testWidgets('a failed render-surface result keeps an error chip', (
    tester,
  ) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [
      const StatusEvent('Calling tool: render-surface', toolCallId: 'call-1'),
      ToolResultEvent.fromJson({
        'tool_name': 'render-surface',
        'status': 'error',
        'tool_call_id': 'call-1',
        'result': 'invalid components',
      }),
    ]);

    final chips = notifier.state.value!.messages
        .where(
          (m) =>
              m.timelineType == TimelineEntryType.toolCall &&
              m.toolCalls.first.toolName == 'render-surface',
        )
        .toList();
    expect(chips, hasLength(1));
    expect(chips.single.toolCalls.first.status, ToolCallStatus.error);

    await _finishTurn(tester, controller);
  });

  testWidgets('a replayed surface_update does not duplicate the entry', (
    tester,
  ) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [_surface(), _surface()]);

    expect(
      notifier.state.value!.messages.where(
        (m) => m.timelineType == TimelineEntryType.surface,
      ),
      hasLength(1),
      reason: 'replay applies the event to the same surface id in place',
    );

    await _finishTurn(tester, controller);
  });

  // -- PR 17: post-turn surface actions --------------------------------------

  testWidgets('an action waits in the queue and shows pending', (tester) async {
    final api = _FakeApiClient();
    final controller = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, controller, [_surface()]);
    final surface = _surfaceEntry(notifier.state.value!)!.surface!;

    unawaited(notifier.submitSurfaceAction(surface, _submission()));
    await tester.pump();

    expect(api.sentMessages, [
      'weather?',
    ], reason: 'the running turn is not interrupted');
    expect(_surfaceEntry(notifier.state.value!)!.status, MessageStatus.sending);

    final actionTurn = api.enqueueStream();
    await _finishTurn(tester, controller);
    await tester.pump();

    expect(api.sentMessages, ['weather?', '']);
    expect(api.sentSurfaceActions.last?.surfaceId, 'surface-1');
    expect(api.sentSurfaceActions.last?.name, 'submit_form');
    expect(_surfaceEntry(notifier.state.value!)!.status, MessageStatus.sending);

    await _deliver(tester, actionTurn, [
      const RunStartedEvent('run-2'),
      _surface(status: 'submitted', messages: const []),
    ]);

    final entry = _surfaceEntry(notifier.state.value!);
    expect(entry!.surface!.status, SurfaceStatus.submitted);
    expect(entry.surface!.answerable, isFalse);
    expect(entry.status, MessageStatus.ok, reason: 'pending clears');

    final viaForm = notifier.state.value!.messages.firstWhere((m) => m.viaForm);
    expect(viaForm.content, 'Submitted “Weather”');

    await _finishTurn(tester, actionTurn);
  });

  testWidgets('a 409 marks the surface submitted and does not retry', (
    tester,
  ) async {
    final api = _FakeApiClient();
    final first = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, first, [_surface()]);
    await _finishTurn(tester, first);
    final surface = _surfaceEntry(notifier.state.value!)!.surface!;

    final actionTurn = api.enqueueStream();
    unawaited(notifier.submitSurfaceAction(surface, _submission()));
    await tester.pump();
    await _deliver(tester, actionTurn, [
      const ErrorEvent('The surface was already submitted', statusCode: 409),
    ]);
    await tester.runAsync(actionTurn.close);
    await tester.pumpAndSettle();

    expect(api.sentMessages, ['weather?', ''], reason: 'no retry');
    final entry = _surfaceEntry(notifier.state.value!);
    expect(entry!.surface!.status, SurfaceStatus.submitted);
    expect(entry.surface!.answerable, isFalse);
    expect(entry.status, MessageStatus.ok);
    expect(notifier.state.value!.error, isNull);
  });

  testWidgets('a 410 marks the surface closed and does not retry', (
    tester,
  ) async {
    final api = _FakeApiClient();
    final first = api.enqueueStream();
    final notifier = await _pump(tester, api);

    unawaited(notifier.sendMessage('weather?'));
    await tester.pump();
    await _deliver(tester, first, [_surface()]);
    await _finishTurn(tester, first);
    final surface = _surfaceEntry(notifier.state.value!)!.surface!;

    final actionTurn = api.enqueueStream();
    unawaited(notifier.submitSurfaceAction(surface, _submission()));
    await tester.pump();
    await _deliver(tester, actionTurn, [
      const ErrorEvent('A later turn started', statusCode: 410),
    ]);
    await tester.runAsync(actionTurn.close);
    await tester.pumpAndSettle();

    expect(api.sentMessages, ['weather?', ''], reason: 'no retry');
    final entry = _surfaceEntry(notifier.state.value!);
    expect(entry!.surface!.status, SurfaceStatus.active);
    expect(entry.surface!.answerable, isFalse);
    expect(entry.surface!.isClosed, isTrue);
    expect(entry.status, MessageStatus.ok);
    expect(notifier.state.value!.error, isNull);
  });
}
