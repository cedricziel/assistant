import 'dart:async';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_ui/models.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';

/// Called when a feedback call failed, with the last server-confirmed value
/// the message must show again.
typedef FeedbackRestore = void Function(MessageFeedback? confirmed);

/// What the feedback controller exposes to the UI.
@immutable
class MessageFeedbackState {
  const MessageFeedbackState({
    this.pending = const {},
    this.commentFields = const {},
  });

  /// Latest optimistic value per message with queued or in-flight calls.
  /// A `null` value means the vote is being cleared.
  final Map<String, MessageFeedback?> pending;

  /// Messages whose comment field is open, with the rating it was opened for.
  final Map<String, MessageRating> commentFields;

  MessageFeedbackState copyWith({
    Map<String, MessageFeedback?>? pending,
    Map<String, MessageRating>? commentFields,
  }) => MessageFeedbackState(
    pending: pending ?? this.pending,
    commentFields: commentFields ?? this.commentFields,
  );
}

class _FeedbackCall {
  _FeedbackCall(this.conversationId, this.feedback, this.onFailed);

  final String conversationId;

  /// The value to store; `null` clears the vote.
  final MessageFeedback? feedback;
  final FeedbackRestore onFailed;
}

class _MessageQueue {
  _MessageQueue(this.confirmed);

  /// The last value the server confirmed (the history value until the first
  /// call succeeds).
  MessageFeedback? confirmed;
  final List<_FeedbackCall> calls = [];
}

/// Sends message feedback to the server in the order the user made it.
///
/// Per message it keeps the last server-confirmed value and a serial queue of
/// calls. A failure drops the rest of the queue and hands the confirmed value
/// back to the caller so it can roll the visible state back to it.
class MessageFeedbackController extends Notifier<MessageFeedbackState> {
  final Map<String, _MessageQueue> _queues = {};

  @override
  MessageFeedbackState build() => const MessageFeedbackState();

  /// Whether a feedback call for [messageId] is queued or in flight.
  bool isPending(String messageId) => state.pending.containsKey(messageId);

  /// The optimistic value for [messageId]; only meaningful while
  /// [isPending].
  MessageFeedback? pendingValue(String messageId) => state.pending[messageId];

  /// The rating the open comment field of [messageId] belongs to, or `null`
  /// when no field is open.
  MessageRating? commentFieldRating(String messageId) =>
      state.commentFields[messageId];

  void openCommentField(String messageId, MessageRating rating) {
    state = state.copyWith(
      commentFields: {...state.commentFields, messageId: rating},
    );
  }

  void closeCommentField(String messageId) {
    if (!state.commentFields.containsKey(messageId)) return;
    state = state.copyWith(
      commentFields: {...state.commentFields}..remove(messageId),
    );
  }

  /// Queue a call that stores [next] for [messageId] ([previous] is the value
  /// shown before this change; it seeds the confirmed value when no call is
  /// pending yet). A `null` [next] deletes the vote.
  void enqueue({
    required String conversationId,
    required String messageId,
    required MessageFeedback? previous,
    required MessageFeedback? next,
    required FeedbackRestore onFailed,
  }) {
    final call = _FeedbackCall(conversationId, next, onFailed);
    final existing = _queues[messageId];
    if (existing != null) {
      existing.calls.add(call);
    } else {
      final queue = _MessageQueue(previous)..calls.add(call);
      _queues[messageId] = queue;
      unawaited(_drain(messageId, queue));
    }
    state = state.copyWith(pending: {...state.pending, messageId: next});
  }

  Future<void> _drain(String messageId, _MessageQueue queue) async {
    while (queue.calls.isNotEmpty) {
      final call = queue.calls.removeAt(0);
      try {
        await _send(messageId, call);
        if (!ref.mounted) return;
        queue.confirmed = call.feedback;
      } catch (_) {
        if (!ref.mounted) return;
        queue.calls.clear();
        _settle(messageId);
        call.onFailed(queue.confirmed);
        return;
      }
    }
    _settle(messageId);
  }

  void _settle(String messageId) {
    _queues.remove(messageId);
    state = state.copyWith(pending: {...state.pending}..remove(messageId));
  }

  Future<void> _send(String messageId, _FeedbackCall call) async {
    final api = ref.read(apiClientProvider);
    if (api == null) throw StateError('Not connected to server');
    final feedback = call.feedback;
    if (feedback == null) {
      await api.conversations.deleteMessageFeedback(
        id: call.conversationId,
        messageId: messageId,
      );
      return;
    }
    await api.conversations.setMessageFeedback(
      id: call.conversationId,
      messageId: messageId,
      setMessageFeedbackRequest: SetMessageFeedbackRequest(
        (b) => b
          ..rating = switch (feedback.rating) {
            MessageRating.positive => FeedbackRating.positive,
            MessageRating.negative => FeedbackRating.negative,
          }
          ..comment = feedback.comment,
      ),
    );
  }
}

/// Provider for [MessageFeedbackController].
///
/// Not auto-disposed: queued calls must outlive the chat notifier's own
/// listeners.
final messageFeedbackControllerProvider =
    NotifierProvider<MessageFeedbackController, MessageFeedbackState>(
      MessageFeedbackController.new,
    );
