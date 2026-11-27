import 'package:assistant_ui/models.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models/interaction_answer.dart';
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
