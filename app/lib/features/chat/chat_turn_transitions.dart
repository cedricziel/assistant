import 'chat_models.dart';
import 'chat_timeline_reducers.dart';

/// State after a turn ends without a final message: the streaming placeholder
/// is dropped and sending stops.
///
/// [failedUserMsgId] marks that user message failed (kept for retry), [error]
/// is surfaced to the UI, and the flags clear the status line and the
/// in-flight turn snapshot.
ChatState abortedTurnState(
  ChatState from, {
  String? failedUserMsgId,
  String? error,
  bool clearStatusMessage = false,
  bool clearTurnStatus = false,
}) {
  final msgs = List<ChatMessage>.from(from.messages)
    ..removeWhere((m) => m.id == streamingPlaceholderId);
  if (failedUserMsgId != null) {
    setMessageStatus(msgs, failedUserMsgId, MessageStatus.failed);
  }
  return from.copyWith(
    messages: msgs,
    isSending: false,
    streamingContent: '',
    error: error,
    clearStatusMessage: clearStatusMessage,
    clearCurrentTurnStatus: clearTurnStatus,
  );
}

/// State after a turn finishes: the placeholder becomes the final assistant
/// message (keeping its audio), streaming thinking entries are completed, and
/// only the conversation, messages and pending queue carry over.
ChatState completedTurnState(
  ChatState from, {
  required String conversationId,
  required String userMsgId,
  required String assistantId,
  required String content,
}) {
  final msgs = List<ChatMessage>.from(from.messages);
  completeStreamingThinkingEntries(msgs);
  setMessageStatus(msgs, userMsgId, MessageStatus.ok);
  final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
  if (idx != -1) {
    final placeholder = msgs[idx];
    msgs[idx] = ChatMessage(
      id: assistantId,
      role: 'assistant',
      content: content,
      audioId: placeholder.audioId,
      ttsAvailable: placeholder.ttsAvailable,
    );
  }
  return ChatState(
    conversationId: conversationId,
    messages: msgs,
    pendingQueue: from.pendingQueue,
  );
}
