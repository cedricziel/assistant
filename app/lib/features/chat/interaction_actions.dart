import 'package:assistant_api/assistant_api.dart' show InteractionDetail;
import 'package:assistant_api/assistant_api.dart' show InteractionStatusView;
import 'package:assistant_ui/models.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models/interaction_answer.dart';
import '../../api/models/stream_event.dart';
import '../../api/models/surface_action.dart';
import '../connection/connection_provider.dart';
import 'chat_models.dart';
import 'chat_timeline_reducers.dart';
import 'interaction_mapping.dart';

/// [state] with [messages], and the turn status carrying the progress label of
/// the interaction the turn now waits on (none clears it).
ChatState withInteractionMessages(ChatState state, List<ChatMessage> messages) {
  final status = state.currentTurnStatus;
  final label = awaitingLabelOf(messages);
  return state.copyWith(
    messages: messages,
    currentTurnStatus: status == null
        ? null
        : label == null
        ? status.copyWith(clearAwaitingLabel: true)
        : status.copyWith(awaitingLabel: label),
  );
}

const _answerFailed =
    "Couldn't send your answer. Check your connection and try again.";

/// Answering interactions, for [ChatNotifier].
///
/// Lives beside the notifier so `chat_provider.dart` does not grow with every
/// interaction kind.
mixin ChatInteractions on AsyncNotifier<ChatState> {
  /// Bumped whenever the active conversation changes, so an answer's async
  /// completion cannot write into the conversation the owner switched to.
  int _conversationEpoch = 0;

  /// Mark the active conversation as changed. Late answer completions from the
  /// previous one are dropped. Called wherever the notifier selects another
  /// conversation, including while its history is still loading.
  void conversationChanged() => _conversationEpoch++;

  /// Whether [epoch] still identifies the active conversation.
  bool _sameConversation(int epoch) =>
      ref.mounted && _conversationEpoch == epoch;

  /// The interactions of [conversationId] that still wait for their owner.
  ///
  /// Opening a conversation checks this list so a card survives a reload, a
  /// push tap on another device and a lost stream. A failure is not fatal: the
  /// history's own interactions still render.
  Future<List<InteractionDetail>> pendingInteractions(
    ApiClient api,
    String conversationId,
  ) async {
    try {
      return await api.listConversationInteractions(
        conversationId,
        status: InteractionStatusView.pending,
      );
    } catch (_) {
      return const [];
    }
  }

  /// Follow [runId]'s replay stream for a conversation opened with a pending
  /// interaction, so its card can be answered and ended.
  ///
  /// Only interaction events are applied: history already carries the tool
  /// calls and text, and replaying them would duplicate entries. The run's
  /// terminal frame calls [onEnded] so the caller can reconcile the final
  /// reply and receipts from history.
  Future<void> watchPendingRun(
    ApiClient api,
    String conversationId,
    String runId, {
    Future<void> Function()? onEnded,
  }) async {
    final epoch = _conversationEpoch;
    try {
      await for (final event in api.streamEventsFrom(
        conversationId,
        runId,
        since: 0,
      )) {
        if (!_sameConversation(epoch)) return;
        final current = state.value;
        if (current == null) return;
        switch (event) {
          case InteractionRequestedEvent():
            final msgs = List<ChatMessage>.from(current.messages);
            applyInteractionRequested(msgs, event);
            state = AsyncData(withInteractionMessages(current, msgs));
          case InteractionResolvedEvent():
            final msgs = List<ChatMessage>.from(current.messages);
            applyInteractionResolved(msgs, event);
            state = AsyncData(withInteractionMessages(current, msgs));
          case DoneEvent() || AgentErrorEvent() || ErrorEvent():
            if (onEnded != null) await onEnded();
            return;
          default:
            // Tokens, tool statuses and thinking are already in history.
            break;
        }
      }
    } catch (_) {
      // The card stays; a later reload finds the stored status.
    }
  }

  /// Send [answer] to the pending interaction [interactionId] of the turn in
  /// flight.
  ///
  /// The card turns to `submitting` and stays there: the `200` only confirms
  /// the server took the answer, and the `interaction_resolved` event on the
  /// stream turns the card into its receipt. A `409` or `410` means it already
  /// ended, so the record is reloaded from the server. Any other failure
  /// restores the pending card and sets [ChatState.error].
  Future<void> respond(String interactionId, InteractionAnswer answer) async {
    final current = state.value;
    final record = current == null
        ? null
        : findInteraction(current.messages, interactionId);
    if (current == null || record == null) return;
    if (record.status != InteractionStatus.pending) return;

    final epoch = _conversationEpoch;
    final api = ref.read(apiClientProvider);
    final conversationId = current.conversationId;
    final runId = current.currentTurnStatus?.runId;
    if (api == null || conversationId == null || runId == null) {
      state = AsyncData(current.copyWith(error: _answerFailed));
      return;
    }

    _setStatus(interactionId, InteractionStatus.submitting);
    try {
      await api.respondToInteraction(
        conversationId,
        runId,
        interactionId,
        answer,
      );
    } on DioException catch (e) {
      if (!_sameConversation(epoch)) return;
      final code = e.response?.statusCode;
      if (code == 409 || code == 410) {
        await _reload(conversationId, interactionId, epoch);
      } else {
        _restorePending(interactionId, epoch);
      }
    } catch (_) {
      if (_sameConversation(epoch)) _restorePending(interactionId, epoch);
    }
  }

  /// Send a surface [action] to the pending `surface_action` interaction
  /// [interactionId] of the turn in flight.
  ///
  /// The surface is the card, so it shows pending while the request runs.
  /// Returns false when the wait is already over (a 410): the interaction is
  /// dropped and the caller sends the same action on the post-turn route.
  /// A `409` reloads the record and is not retried; any other failure restores
  /// the pending state and shows [ChatState.error].
  Future<bool> respondSurfaceAction(
    String interactionId,
    SurfaceActionSubmission action,
  ) async {
    final current = state.value;
    final record = current == null
        ? null
        : findInteraction(current.messages, interactionId);
    final api = ref.read(apiClientProvider);
    final conversationId = current?.conversationId;
    final runId = current?.currentTurnStatus?.runId;
    if (current == null ||
        record == null ||
        record.status != InteractionStatus.pending ||
        api == null ||
        conversationId == null ||
        runId == null) {
      return true;
    }

    final epoch = _conversationEpoch;
    _setStatus(interactionId, InteractionStatus.submitting);
    try {
      await api.respondToInteraction(
        conversationId,
        runId,
        interactionId,
        SurfaceActionAnswer(action),
      );
      return true;
    } on DioException catch (e) {
      if (!_sameConversation(epoch)) return true;
      switch (e.response?.statusCode) {
        case 410:
          _endSurfaceInteraction(interactionId, epoch);
          return false;
        case 409:
          await _reload(conversationId, interactionId, epoch);
          _clearSurfaceInteraction(interactionId, epoch);
          return true;
        default:
          _restorePending(interactionId, epoch);
          return true;
      }
    } catch (_) {
      if (_sameConversation(epoch)) _restorePending(interactionId, epoch);
      return true;
    }
  }

  /// Drop an interaction the turn no longer waits on: it expired or was
  /// cancelled before the answer arrived.
  void _endSurfaceInteraction(String interactionId, int epoch) {
    if (!_sameConversation(epoch)) return;
    final current = state.value;
    if (current == null) return;
    final msgs = List<ChatMessage>.from(current.messages);
    updateInteraction(
      msgs,
      interactionId,
      (r) => r.copyWith(status: InteractionStatus.expired),
    );
    clearSurfaceInteractions(msgs, interactionId: interactionId);
    state = AsyncData(withInteractionMessages(current, msgs));
  }

  void _clearSurfaceInteraction(String interactionId, int epoch) {
    if (!_sameConversation(epoch)) return;
    final current = state.value;
    if (current == null) return;
    final msgs = List<ChatMessage>.from(current.messages);
    clearSurfaceInteractions(msgs, interactionId: interactionId);
    state = AsyncData(withInteractionMessages(current, msgs));
  }

  void _setStatus(String id, InteractionStatus status) {
    final current = state.value;
    if (current == null) return;
    final msgs = List<ChatMessage>.from(current.messages);
    updateInteraction(msgs, id, (r) => r.copyWith(status: status));
    state = AsyncData(withInteractionMessages(current, msgs));
  }

  /// Put a card back to pending after a failed request, unless an event has
  /// already ended it.
  void _restorePending(String id, int epoch) {
    if (!_sameConversation(epoch)) return;
    final current = state.value;
    if (current == null) return;
    final msgs = List<ChatMessage>.from(current.messages);
    if (findInteraction(msgs, id)?.status == InteractionStatus.submitting) {
      updateInteraction(
        msgs,
        id,
        (r) => r.copyWith(status: InteractionStatus.pending),
      );
    }
    state = AsyncData(
      withInteractionMessages(current, msgs).copyWith(error: _answerFailed),
    );
  }

  Future<void> _reload(String conversationId, String id, int epoch) async {
    try {
      final details = await ref
          .read(apiClientProvider)
          ?.listConversationInteractions(conversationId);
      if (!_sameConversation(epoch)) return;
      final detail = details?.where((d) => d.interactionId == id).firstOrNull;
      final current = state.value;
      if (current == null) return;
      if (detail == null) return _restorePending(id, epoch);
      final msgs = List<ChatMessage>.from(current.messages);
      final reloaded = interactionRecordFromDetail(detail);
      updateInteraction(msgs, id, (_) => reloaded);
      state = AsyncData(withInteractionMessages(current, msgs));
    } catch (_) {
      if (_sameConversation(epoch)) _restorePending(id, epoch);
    }
  }
}
