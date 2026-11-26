import 'package:assistant_ui/models.dart';

// -- Active chat state -------------------------------------------------------

/// A completed tool result recorded during streaming — used by
/// [AgentEventListener] to fire skill notifications.
class ChatToolResult {
  const ChatToolResult({required this.toolName, required this.status});

  final String toolName;

  /// `"ok"` on success, `"error"` or `"denied"` otherwise.
  final String status;

  bool get isSuccess => status == 'ok';
}

/// State for the active chat.
class ChatState {
  const ChatState({
    this.conversationId,
    this.messages = const [],
    this.isSending = false,
    this.isStopping = false,
    this.isLoadingHistory = false,
    this.isReconnecting = false,
    this.streamingContent = '',
    this.statusMessage,
    this.lastToolResult,
    this.error,
    this.pendingQueue = const [],
    this.currentTurnStatus,
  });

  final String? conversationId;
  final List<ChatMessage> messages;
  final bool isSending;

  /// True from a Stop request until the turn's terminal frame is processed.
  /// Further Stop requests are ignored while set.
  final bool isStopping;
  final bool isLoadingHistory;

  /// True while [ChatNotifier.attemptReconnect] is actively recovering an
  /// interrupted stream — drives the "Reconnecting…" banner.
  /// Distinct from [isSending]: reconnect happens after the original
  /// stream terminated, before its replay reaches DoneEvent.
  final bool isReconnecting;

  /// Accumulated token content during streaming (before DoneEvent).
  final String streamingContent;

  /// Transient status text from the assistant (e.g. "Calling tool: web-search").
  /// Cleared when the stream completes.
  final String? statusMessage;

  /// The most recent tool result received during streaming; changes trigger
  /// skill completion notifications via [AgentEventListener].
  final ChatToolResult? lastToolResult;

  final String? error;

  /// Messages waiting to be sent after the current in-flight response completes.
  final List<PendingMessage> pendingQueue;

  /// Snapshot of the in-flight turn's most recent activity, exposed for
  /// rendering progress indicators above the composer. `null` when no
  /// turn is in flight (between `done` and the next `run_started`).
  final TurnStatusSnapshot? currentTurnStatus;

  bool get isStreaming => isSending && streamingContent.isNotEmpty;

  ChatState copyWith({
    String? conversationId,
    List<ChatMessage>? messages,
    bool? isSending,
    bool? isStopping,
    bool? isLoadingHistory,
    bool? isReconnecting,
    String? streamingContent,
    String? statusMessage,
    ChatToolResult? lastToolResult,
    String? error,
    bool clearError = false,
    bool clearConversation = false,
    bool clearStatusMessage = false,
    List<PendingMessage>? pendingQueue,
    TurnStatusSnapshot? currentTurnStatus,
    bool clearCurrentTurnStatus = false,
  }) {
    return ChatState(
      conversationId: clearConversation
          ? null
          : (conversationId ?? this.conversationId),
      messages: messages ?? this.messages,
      isSending: isSending ?? this.isSending,
      isStopping: isStopping ?? this.isStopping,
      isLoadingHistory: isLoadingHistory ?? this.isLoadingHistory,
      isReconnecting: isReconnecting ?? this.isReconnecting,
      streamingContent: streamingContent ?? this.streamingContent,
      statusMessage: clearStatusMessage
          ? null
          : (statusMessage ?? this.statusMessage),
      lastToolResult: lastToolResult ?? this.lastToolResult,
      error: clearError ? null : (error ?? this.error),
      pendingQueue: pendingQueue ?? this.pendingQueue,
      currentTurnStatus: clearCurrentTurnStatus
          ? null
          : (currentTurnStatus ?? this.currentTurnStatus),
    );
  }
}
