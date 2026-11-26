import 'package:assistant_ui/models.dart';

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
    isStopping: false,
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

/// State after a turn ends with a cancelled `agent_error` frame.
///
/// The streaming placeholder becomes a [MessageStatus.stopped] assistant
/// message holding [partialContent] (the streamed text when that is empty),
/// under [messageId] or a synthetic id. With no text and no [messageId] the
/// placeholder is dropped. Pending tool calls become
/// [ToolCallStatus.cancelled] and active timeline entries complete. The user
/// message is `ok`, or `failed` (retryable) when the server never persisted
/// it ([userMessagePersisted] false); then no reply is kept. Timeline entries
/// and the pending queue carry over.
ChatState stoppedTurnState(
  ChatState from, {
  required String userMsgId,
  required String partialContent,
  String? messageId,
  bool userMessagePersisted = true,
}) {
  final msgs = List<ChatMessage>.from(from.messages);
  completeActiveTimelineEntries(msgs);
  for (final m in msgs) {
    for (final record in m.toolCalls) {
      if (record.status == ToolCallStatus.pending) {
        record.status = ToolCallStatus.cancelled;
      }
    }
  }
  setMessageStatus(
    msgs,
    userMsgId,
    userMessagePersisted ? MessageStatus.ok : MessageStatus.failed,
  );

  final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
  if (idx != -1) {
    final placeholder = msgs[idx];
    final content = partialContent.isNotEmpty
        ? partialContent
        : from.streamingContent;
    if (!userMessagePersisted || (content.isEmpty && messageId == null)) {
      msgs.removeAt(idx);
    } else {
      msgs[idx] = ChatMessage(
        id: messageId ?? 'assistant-${DateTime.now().millisecondsSinceEpoch}',
        role: 'assistant',
        content: content,
        status: MessageStatus.stopped,
        audioId: placeholder.audioId,
        ttsAvailable: placeholder.ttsAvailable,
      );
    }
  }
  return from.copyWith(
    messages: msgs,
    isSending: false,
    isStopping: false,
    streamingContent: '',
    clearError: true,
    clearStatusMessage: true,
    clearCurrentTurnStatus: true,
  );
}
