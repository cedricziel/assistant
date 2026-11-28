// ChatNotifier handling of turn interactions: a tool approval attaches to its
// tool call, `respond()` answers it, and the resolved event ends it.

import 'dart:async';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart'
    show InteractionDetail, InteractionStatusView, TurnState;
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/interaction_answer.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/api/models/surface_action.dart';
import 'package:assistant_app/features/chat/chat_announcer.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:built_value/json_object.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _FakeApi extends ApiClient {
  _FakeApi() : super(baseUrl: 'http://fake', token: 'fake');

  final stream = StreamController<StreamEvent>();

  @override
  Stream<StreamEvent> streamMessages(
    String conversationId,
    String message, {
    List<String>? attachmentIds,
    SurfaceActionSubmission? surfaceAction,
  }) => stream.stream;

  @override
  Stream<StreamEvent> sendVoiceMessage(
    String conversationId,
    Uint8List audioBytes,
    String mimeType,
  ) => stream.stream;

  /// Every replay subscription opened, in call order.
  final replays = <StreamController<StreamEvent>>[];

  @override
  Stream<StreamEvent> streamEventsFrom(
    String conversationId,
    String runId, {
    int since = 0,
    CancelToken? cancelToken,
  }) {
    final replay = StreamController<StreamEvent>();
    replays.add(replay);
    return replay.stream;
  }

  final responses = <({String turnId, String interactionId, Object answer})>[];

  /// Failure to throw from the next [respondToInteraction]; null answers.
  DioException? respondError;
  Completer<void>? respondGate;

  @override
  Future<InteractionDetail> respondToInteraction(
    String conversationId,
    String turnId,
    String interactionId,
    InteractionAnswer answer,
  ) async {
    responses.add((
      turnId: turnId,
      interactionId: interactionId,
      answer: answer,
    ));
    await respondGate?.future;
    final error = respondError;
    if (error != null) throw error;
    return _detail(interactionId, InteractionStatusView.resolved);
  }

  final cancelled = <String>[];

  @override
  Future<TurnState?> cancelTurn(String conversationId, String turnId) async {
    cancelled.add(turnId);
    return TurnState.running;
  }

  /// What [listConversationInteractions] reports.
  List<InteractionDetail> listed = [];

  @override
  Future<List<InteractionDetail>> listConversationInteractions(
    String conversationId, {
    InteractionStatusView? status,
  }) async => listed;
}

InteractionDetail _detail(String id, InteractionStatusView status) =>
    InteractionDetail(
      (b) => b
        ..conversationId = 'conv-1'
        ..interactionId = id
        ..kind = 'tool_approval'
        ..rendersCard = true
        ..runId = 'run-1'
        ..status = status
        ..toolCallId = 'call-1'
        ..createdAt = DateTime.utc(2026)
        ..expiresAt = DateTime.utc(2026, 1, 1, 0, 10)
        ..resolvedAt = status == InteractionStatusView.pending
            ? null
            : DateTime.utc(2026, 1, 1, 0, 2)
        ..response = status == InteractionStatusView.resolved
            ? JsonObject({'decision': 'reject_once'})
            : null
        ..request = JsonObject({
          'tool_name': 'bash',
          'arguments': {'command': 'git status'},
          'options': ['allow_once', 'reject_once'],
        }),
    );

DioException _http(int status) => DioException(
  requestOptions: RequestOptions(path: ''),
  response: Response<void>(
    requestOptions: RequestOptions(path: ''),
    statusCode: status,
  ),
  type: DioExceptionType.badResponse,
);

InteractionRequestedEvent _approval({
  String id = 'int-1',
  String? toolCallId = 'call-1',
  String kind = 'tool_approval',
  bool rendersCard = true,
}) => InteractionRequestedEvent(
  interactionId: id,
  kind: kind,
  createdAt: DateTime.utc(2026),
  expiresAt: DateTime.utc(2026, 1, 1, 0, 10),
  toolCallId: toolCallId,
  rendersCard: rendersCard,
  request: const {
    'tool_name': 'bash',
    'arguments': {'command': 'git status --short'},
    'arguments_truncated': true,
    'reason': 'Shell commands need approval',
    'options': ['allow_once', 'allow_always', 'reject_once'],
    'suggested_rule': 'bash(git status*)',
  },
);

InteractionResolvedEvent _resolved({
  String id = 'int-1',
  InteractionOutcome outcome = InteractionOutcome.resolved,
  Map<String, Object?>? response = const {'decision': 'allow_always'},
  String? ruleId = 'rule-1',
}) => InteractionResolvedEvent(
  interactionId: id,
  kind: 'tool_approval',
  outcome: outcome,
  resolvedAt: DateTime.utc(2026, 1, 1, 0, 1),
  response: outcome == InteractionOutcome.resolved ? response : null,
  ruleId: ruleId,
);

class _Host extends ConsumerWidget {
  const _Host({required this.onBuild});
  final void Function(ChatNotifier) onBuild;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    onBuild(ref.watch(chatProvider.notifier));
    return const MaterialApp(home: Scaffold());
  }
}

class _Fixture {
  _Fixture(this.tester, this.api, this.notifier);

  final WidgetTester tester;
  final _FakeApi api;
  final ChatNotifier notifier;
  final lifecycle = <ChatLifecycleEvent>[];

  ChatState get state => notifier.state.value!;

  Future<void> emit(StreamEvent e) async {
    await tester.runAsync(() async {
      api.stream.add(e);
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);
    });
    await tester.pump();
  }

  /// The tool call entry named `bash`, or null.
  ChatMessage? get toolEntry => state.messages
      .where((m) => m.timelineType == TimelineEntryType.toolCall)
      .firstOrNull;

  InteractionRecord? get toolInteraction =>
      toolEntry?.toolCalls.first.interaction;

  Future<void> settle() async {
    await tester.runAsync(() => Future<void>.delayed(Duration.zero));
    await tester.pump();
  }
}

Future<_Fixture> _start(WidgetTester tester, {bool voice = false}) async {
  final api = _FakeApi();
  late ChatNotifier notifier;
  await tester.pumpWidget(
    ProviderScope(
      overrides: [apiClientProvider.overrideWithValue(api)],
      child: _Host(onBuild: (n) => notifier = n),
    ),
  );
  await tester.pump();
  notifier.setConversationId('conv-1');
  final f = _Fixture(tester, api, notifier);
  notifier.lifecycle.listen(f.lifecycle.add);
  if (voice) {
    unawaited(notifier.sendVoiceMessage(Uint8List(0), 'audio/wav'));
  } else {
    unawaited(notifier.sendMessage('go'));
  }
  await tester.pump();
  await f.emit(const RunStartedEvent('run-1'));
  return f;
}

void main() {
  group('interaction_requested', () {
    testWidgets('with a tool_call_id attaches to that tool call entry', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(
        const StatusEvent('Calling tool: bash', toolCallId: 'call-1'),
      );
      await f.emit(_approval());

      final record = f.toolInteraction!;
      expect(record.id, 'int-1');
      expect(record.kind, InteractionKind.toolApproval);
      expect(record.status, InteractionStatus.pending);
      expect(record.toolName, 'bash');
      expect(record.arguments, {'command': 'git status --short'});
      expect(record.argumentsTruncated, isTrue);
      expect(record.reason, 'Shell commands need approval');
      expect(record.suggestedRule, 'bash(git status*)');
      expect(record.options, [
        ApprovalDecision.allowOnce,
        ApprovalDecision.allowAlways,
        ApprovalDecision.rejectOnce,
      ]);
      expect(
        f.state.messages.where(
          (m) => m.timelineType == TimelineEntryType.interaction,
        ),
        isEmpty,
        reason: 'no separate entry when the tool call exists',
      );
      expect(
        f.state.currentTurnStatus?.awaitingLabel,
        'Waiting for your approval',
      );
      expect(f.state.lastInteractionRequest?.id, 'int-1');
      unawaited(f.api.stream.close());
    });

    testWidgets('without a tool_call_id adds an interaction entry in order', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('Calling tool: bash'));
      await f.emit(_approval(toolCallId: null));

      final types = f.state.messages.map((m) => m.timelineType).toList();
      final entry = f.state.messages.firstWhere(
        (m) => m.timelineType == TimelineEntryType.interaction,
      );
      expect(entry.interaction?.id, 'int-1');
      expect(
        types.indexOf(TimelineEntryType.interaction),
        greaterThan(types.indexOf(TimelineEntryType.toolCall)),
      );
      expect(
        f.state.messages.last.id,
        streamingPlaceholderId,
        reason: 'entries stay above the reply placeholder',
      );
      unawaited(f.api.stream.close());
    });

    testWidgets('a tool_call_id with no matching entry falls back too', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(_approval(toolCallId: 'call-unknown'));

      expect(
        f.state.messages.any(
          (m) => m.timelineType == TimelineEntryType.interaction,
        ),
        isTrue,
      );
      unawaited(f.api.stream.close());
    });

    testWidgets('an unregistered kind waits with the generic label', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval(kind: 'hologram', rendersCard: false));

      expect(f.toolInteraction?.kind, InteractionKind.unknown);
      expect(f.toolInteraction?.rendersCard, isFalse);
      expect(
        f.state.currentTurnStatus?.awaitingLabel,
        'Waiting for your input',
      );
      unawaited(f.api.stream.close());
    });

    testWidgets('announces approval needed, then resolution', (tester) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());

      final awaiting = f.lifecycle.whereType<TurnAwaitingInput>().single;
      expect(awaiting.announcement, 'Approval needed');

      await f.emit(_resolved());
      expect(f.lifecycle.whereType<TurnInputResolved>(), hasLength(1));
      unawaited(f.api.stream.close());
    });

    testWidgets('an unregistered kind ignores malformed approval fields', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(
        InteractionRequestedEvent(
          interactionId: 'int-q',
          kind: 'question',
          createdAt: DateTime.utc(2026),
          expiresAt: DateTime.utc(2026, 1, 1, 0, 10),
          request: const {
            'tool_name': 42,
            'reason': false,
            'suggested_rule': ['bash'],
            'options': 7,
          },
        ),
      );

      final entry = f.state.messages.firstWhere(
        (m) => m.timelineType == TimelineEntryType.interaction,
      );
      expect(entry.interaction?.toolName, isNull);
      expect(entry.interaction?.reason, isNull);
      expect(entry.interaction?.suggestedRule, isNull);
      expect(entry.interaction?.options, isEmpty);
      unawaited(f.api.stream.close());
    });
  });

  group('interaction_resolved', () {
    testWidgets('flips the record to a receipt and clears the label', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());
      await f.emit(_resolved());

      final record = f.toolInteraction!;
      expect(record.status, InteractionStatus.resolved);
      expect(record.decision, ApprovalDecision.allowAlways);
      expect(record.ruleId, 'rule-1');
      expect(f.state.currentTurnStatus?.awaitingLabel, isNull);
      unawaited(f.api.stream.close());
    });

    testWidgets('expired and cancelled outcomes end the card', (tester) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());
      await f.emit(_resolved(outcome: InteractionOutcome.expired));
      expect(f.toolInteraction?.status, InteractionStatus.expired);

      await f.emit(const StatusEvent('y', toolCallId: 'call-2'));
      await f.emit(_approval(id: 'int-2', toolCallId: 'call-2'));
      await f.emit(
        _resolved(id: 'int-2', outcome: InteractionOutcome.cancelled),
      );
      final second = f.state.messages
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .last
          .toolCalls
          .first
          .interaction;
      expect(second?.status, InteractionStatus.cancelled);
      unawaited(f.api.stream.close());
    });
  });

  group('respond()', () {
    Future<_Fixture> pending(WidgetTester tester) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());
      return f;
    }

    testWidgets('marks submitting, posts the answer, waits for the event', (
      tester,
    ) async {
      final f = await pending(tester);
      f.api.respondGate = Completer<void>();

      final future = f.notifier.respond(
        'int-1',
        const ToolApprovalAnswer(ApprovalDecision.allowOnce),
      );
      await f.settle();
      expect(f.toolInteraction?.status, InteractionStatus.submitting);
      expect(f.api.responses.single.turnId, 'run-1');
      expect(f.api.responses.single.interactionId, 'int-1');
      expect(
        (f.api.responses.single.answer as ToolApprovalAnswer).decision,
        ApprovalDecision.allowOnce,
      );

      f.api.respondGate!.complete();
      await tester.runAsync(() => future);
      await tester.pump();
      expect(
        f.toolInteraction?.status,
        InteractionStatus.submitting,
        reason: 'the resolved event, not the 200, ends the card',
      );

      await f.emit(
        _resolved(response: const {'decision': 'allow_once'}, ruleId: null),
      );
      expect(f.toolInteraction?.status, InteractionStatus.resolved);
      expect(f.toolInteraction?.decision, ApprovalDecision.allowOnce);
      unawaited(f.api.stream.close());
    });

    testWidgets('ignores a second tap while submitting', (tester) async {
      final f = await pending(tester);
      f.api.respondGate = Completer<void>();

      unawaited(
        f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.allowOnce),
        ),
      );
      await f.settle();
      unawaited(
        f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.rejectOnce),
        ),
      );
      await f.settle();

      expect(f.api.responses, hasLength(1));
      f.api.respondGate!.complete();
      unawaited(f.api.stream.close());
    });

    testWidgets('409 reloads the record from the server', (tester) async {
      final f = await pending(tester);
      f.api.respondError = _http(409);
      f.api.listed = [_detail('int-1', InteractionStatusView.resolved)];

      await tester.runAsync(
        () => f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.allowOnce),
        ),
      );
      await tester.pump();

      expect(f.toolInteraction?.status, InteractionStatus.resolved);
      expect(f.toolInteraction?.decision, ApprovalDecision.rejectOnce);
      expect(f.state.error, isNull);
      unawaited(f.api.stream.close());
    });

    testWidgets('410 shows the interaction as expired', (tester) async {
      final f = await pending(tester);
      f.api.respondError = _http(410);
      f.api.listed = [_detail('int-1', InteractionStatusView.expired)];

      await tester.runAsync(
        () => f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.allowOnce),
        ),
      );
      await tester.pump();

      expect(f.toolInteraction?.status, InteractionStatus.expired);
      unawaited(f.api.stream.close());
    });

    testWidgets('a failed request restores pending and shows an error', (
      tester,
    ) async {
      final f = await pending(tester);
      f.api.respondError = DioException(
        requestOptions: RequestOptions(path: ''),
        type: DioExceptionType.connectionError,
      );

      await tester.runAsync(
        () => f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.allowOnce),
        ),
      );
      await tester.pump();

      expect(f.toolInteraction?.status, InteractionStatus.pending);
      expect(f.state.error, isNotNull);
      unawaited(f.api.stream.close());
    });

    testWidgets('a failed answer does not write into the next conversation', (
      tester,
    ) async {
      final f = await pending(tester);
      f.api.respondGate = Completer<void>();
      f.api.respondError = DioException(
        requestOptions: RequestOptions(path: ''),
        type: DioExceptionType.connectionError,
      );

      final future = f.notifier.respond(
        'int-1',
        const ToolApprovalAnswer(ApprovalDecision.allowOnce),
      );
      await f.settle();
      f.notifier.setConversationId('conv-2');
      await f.settle();

      f.api.respondGate!.complete();
      await tester.runAsync(() => future);
      await tester.pump();

      expect(
        f.state.error,
        isNull,
        reason: 'the answer belonged to the previous conversation',
      );
      unawaited(f.api.stream.close());
    });

    testWidgets('a failed answer does not write into a cleared chat', (
      tester,
    ) async {
      final f = await pending(tester);
      f.api.respondGate = Completer<void>();
      f.api.respondError = DioException(
        requestOptions: RequestOptions(path: ''),
        type: DioExceptionType.connectionError,
      );

      final future = f.notifier.respond(
        'int-1',
        const ToolApprovalAnswer(ApprovalDecision.allowOnce),
      );
      await f.settle();
      f.notifier.clearConversation();
      await f.settle();

      f.api.respondGate!.complete();
      await tester.runAsync(() => future);
      await tester.pump();

      expect(
        f.state.error,
        isNull,
        reason: 'the cleared chat is not the conversation that answered',
      );
      unawaited(f.api.stream.close());
    });
  });

  group('Stop while waiting', () {
    testWidgets('asks the server to cancel, which cancels the card', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());

      f.notifier.stopTurn();
      await f.settle();
      expect(f.api.cancelled, ['run-1']);
      expect(f.toolInteraction?.status, InteractionStatus.pending);

      await f.emit(_resolved(outcome: InteractionOutcome.cancelled));
      expect(f.toolInteraction?.status, InteractionStatus.cancelled);
      unawaited(f.api.stream.close());
    });

    testWidgets('a cancelled turn ends the pending card as cancelled', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());

      await f.emit(
        const AgentErrorEvent(
          reason: AgentErrorReason.cancelled,
          message: 'Turn cancelled',
          partialContent: '',
        ),
      );

      expect(f.toolInteraction?.status, InteractionStatus.cancelled);
      expect(f.state.currentTurnStatus, isNull);
      unawaited(f.api.stream.close());
    });
  });

  group('voice turns', () {
    testWidgets('a voice turn opens the status its approval needs', (
      tester,
    ) async {
      final f = await _start(tester, voice: true);
      expect(
        f.state.currentTurnStatus?.runId,
        'run-1',
        reason: 'the voice path opens the turn-status snapshot too',
      );

      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());
      expect(
        f.state.currentTurnStatus?.awaitingLabel,
        'Waiting for your approval',
      );

      await tester.runAsync(
        () => f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.allowOnce),
        ),
      );
      await tester.pump();
      expect(f.api.responses, hasLength(1));
      expect(f.api.responses.single.turnId, 'run-1');
      unawaited(f.api.stream.close());
    });
  });

  group('replay', () {
    testWidgets('restores the turn status for a retained approval', (
      tester,
    ) async {
      final f = await _start(tester);
      await f.emit(const StatusEvent('x', toolCallId: 'call-1'));
      await f.emit(_approval());
      final userMessage = f.state.messages.firstWhere((m) => m.role == 'user');

      // The stream ends without `done`, so the turn status is dropped while
      // the approval stays on the timeline.
      await tester.runAsync(f.api.stream.close);
      await tester.pump();
      expect(f.state.currentTurnStatus, isNull);

      unawaited(f.notifier.retryMessage(userMessage));
      await f.settle();
      expect(f.api.replays, hasLength(1), reason: 'one retry opens one replay');

      expect(
        f.state.currentTurnStatus?.runId,
        'run-1',
        reason: 'restored from the cursor, without a run_started frame',
      );
      expect(
        f.state.currentTurnStatus?.awaitingLabel,
        'Waiting for your approval',
      );

      await tester.runAsync(
        () => f.notifier.respond(
          'int-1',
          const ToolApprovalAnswer(ApprovalDecision.allowOnce),
        ),
      );
      await tester.pump();
      expect(f.api.responses, hasLength(1));

      // End the resumed turn so it cancels its stall timer.
      await tester.runAsync(() async {
        f.api.replays.last.add(
          const DoneEvent(
            role: 'assistant',
            content: 'ok',
            messageId: 'm1',
            sequenceId: 5,
          ),
        );
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      });
      await tester.pump();
      for (final r in f.api.replays) {
        unawaited(r.close());
      }
    });
  });
}
