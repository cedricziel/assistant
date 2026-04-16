import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';

// -- Conversation list -------------------------------------------------------

/// State for the conversation list.
class ConversationListState {
  const ConversationListState({
    this.conversations = const [],
    this.isLoading = false,
    this.error,
  });

  final List<ConversationSummary> conversations;
  final bool isLoading;
  final String? error;

  ConversationListState copyWith({
    List<ConversationSummary>? conversations,
    bool? isLoading,
    String? error,
    bool clearError = false,
  }) {
    return ConversationListState(
      conversations: conversations ?? this.conversations,
      isLoading: isLoading ?? this.isLoading,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Manages the list of conversations.
class ConversationListNotifier extends AsyncNotifier<ConversationListState> {
  @override
  Future<ConversationListState> build() async {
    // Watch so we rebuild reactively when the API client becomes available
    // (e.g. after the active context loads asynchronously on first launch).
    final api = ref.watch(apiClientProvider);
    if (api == null) return const ConversationListState();

    try {
      final response = await api.conversations.listConversations();
      final conversations = response.data!.toList();
      return ConversationListState(conversations: conversations);
    } catch (e) {
      return ConversationListState(error: e.toString());
    }
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  /// Reload conversations from the server.
  Future<void> refresh() async {
    final api = _api;
    if (api == null) return;

    state = const AsyncLoading();
    try {
      final response = await api.conversations.listConversations();
      final conversations = response.data!.toList();
      state = AsyncData(ConversationListState(conversations: conversations));
    } catch (e) {
      state = AsyncData(ConversationListState(error: e.toString()));
    }
  }

  /// Create a new conversation and add it to the list.
  Future<ConversationSummary?> createConversation({String? title}) async {
    final api = _api;
    if (api == null) return null;

    try {
      final response = await api.conversations.createConversation(
        createConversationRequest: CreateConversationRequest(
          (b) => b.title = title ?? 'New Chat',
        ),
      );
      final created = response.data!;
      final current = state.value ?? const ConversationListState();
      state = AsyncData(
        current.copyWith(conversations: [created, ...current.conversations]),
      );
      return created;
    } catch (e) {
      return null;
    }
  }

  /// Add an already-created conversation to the front of the local list.
  ///
  /// Unlike [createConversation], this does NOT make a network call — it only
  /// updates local state. Use this when the conversation was already created
  /// by a different code path (e.g. [ChatNotifier.sendMessage]).
  void prependConversation(ConversationSummary conv) {
    final current = state.value ?? const ConversationListState();
    state = AsyncData(
      current.copyWith(conversations: [conv, ...current.conversations]),
    );
  }

  /// Delete a conversation by ID.
  Future<void> deleteConversation(String id) async {
    final api = _api;
    if (api == null) return;

    await api.conversations.deleteConversation(id: id);
    final current = state.value ?? const ConversationListState();
    state = AsyncData(
      current.copyWith(
        conversations: current.conversations.where((c) => c.id != id).toList(),
      ),
    );
  }
}

/// Provider for [ConversationListNotifier].
final conversationListProvider =
    AsyncNotifierProvider.autoDispose<
      ConversationListNotifier,
      ConversationListState
    >(ConversationListNotifier.new);

// -- Active chat state -------------------------------------------------------

/// Delivery status of a user message.
enum MessageStatus {
  /// The message is currently in-flight (streaming response in progress).
  sending,

  /// The message was acknowledged by the server (DoneEvent received).
  ok,

  /// The message failed to deliver (network or server error).
  failed,
}

/// A message shown in the chat UI (may be a streaming partial).
class ChatMessage {
  ChatMessage({
    required this.id,
    required this.role,
    required this.content,
    this.isStreaming = false,
    this.status = MessageStatus.ok,
    this.audioId,
  });

  final String id;
  final String role;
  String content;
  bool isStreaming;
  MessageStatus status;

  /// Non-null when the server has produced audio for this assistant message.
  final String? audioId;

  bool get isUser => role == 'user';
  bool get isAssistant => role == 'assistant';

  ChatMessage copyWith({
    String? content,
    bool? isStreaming,
    MessageStatus? status,
    String? audioId,
  }) {
    return ChatMessage(
      id: id,
      role: role,
      content: content ?? this.content,
      isStreaming: isStreaming ?? this.isStreaming,
      status: status ?? this.status,
      audioId: audioId ?? this.audioId,
    );
  }
}

/// A message waiting in the send queue.
///
/// Stores the message text together with the [conversationId] that was active
/// at enqueue time, so messages are always routed to the correct conversation
/// even if the user navigates away while the queue is draining.
class PendingMessage {
  const PendingMessage({required this.text, required this.conversationId});
  final String text;
  final String conversationId;
}

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
    this.isLoadingHistory = false,
    this.streamingContent = '',
    this.statusMessage,
    this.lastToolResult,
    this.error,
    this.pendingQueue = const [],
  });

  final String? conversationId;
  final List<ChatMessage> messages;
  final bool isSending;
  final bool isLoadingHistory;

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

  bool get isStreaming => isSending && streamingContent.isNotEmpty;

  ChatState copyWith({
    String? conversationId,
    List<ChatMessage>? messages,
    bool? isSending,
    bool? isLoadingHistory,
    String? streamingContent,
    String? statusMessage,
    ChatToolResult? lastToolResult,
    String? error,
    bool clearError = false,
    bool clearConversation = false,
    bool clearStatusMessage = false,
    List<PendingMessage>? pendingQueue,
  }) {
    return ChatState(
      conversationId: clearConversation
          ? null
          : (conversationId ?? this.conversationId),
      messages: messages ?? this.messages,
      isSending: isSending ?? this.isSending,
      isLoadingHistory: isLoadingHistory ?? this.isLoadingHistory,
      streamingContent: streamingContent ?? this.streamingContent,
      statusMessage: clearStatusMessage
          ? null
          : (statusMessage ?? this.statusMessage),
      lastToolResult: lastToolResult ?? this.lastToolResult,
      error: clearError ? null : (error ?? this.error),
      pendingQueue: pendingQueue ?? this.pendingQueue,
    );
  }
}

/// Manages the active chat conversation.
class ChatNotifier extends AsyncNotifier<ChatState> {
  bool _cancelled = false;
  bool _draining = false;

  @override
  Future<ChatState> build() async {
    return const ChatState();
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  /// Cancel the current in-progress streaming response.
  ///
  /// Queued messages in [ChatState.pendingQueue] are preserved and will
  /// continue to drain after the cancel.
  void cancelStream() {
    _cancelled = true;
    final current = state.value ?? const ChatState();
    final msgs = List<ChatMessage>.from(current.messages)
      ..removeWhere((m) => m.id == 'assistant-streaming');
    state = AsyncData(
      current.copyWith(
        messages: msgs,
        isSending: false,
        streamingContent: '',
        clearStatusMessage: true,
      ),
    );
  }

  /// Load an existing conversation by ID.
  Future<void> loadConversation(String conversationId) async {
    state = AsyncData(
      (state.value ?? const ChatState()).copyWith(
        isLoadingHistory: true,
        clearError: true,
      ),
    );

    final api = _api;
    if (api == null) {
      state = AsyncData(const ChatState(error: 'Not connected to server'));
      return;
    }

    try {
      final response = await api.conversations.getConversation(
        id: conversationId,
      );
      final detail = response.data!;
      final messages = detail.messages
          .map((m) => ChatMessage(id: m.id, role: m.role, content: m.content))
          .toList();

      state = AsyncData(
        ChatState(conversationId: conversationId, messages: messages),
      );
    } catch (e) {
      state = AsyncData(
        ChatState(
          conversationId: conversationId,
          error: 'Failed to load conversation: $e',
        ),
      );
    }
  }

  /// Start a fresh conversation (no conversation ID set).
  void clearConversation() {
    state = const AsyncData(ChatState());
  }

  /// Dismiss the current error without affecting messages or conversation.
  void dismissError() {
    final current = state.value ?? const ChatState();
    state = AsyncData(current.copyWith(clearError: true));
  }

  /// Set the active conversation ID without loading history.
  void setConversationId(String id) {
    state = AsyncData(
      (state.value ?? const ChatState()).copyWith(conversationId: id),
    );
  }

  /// Enqueue [message] for sending.
  ///
  /// If no drain is currently in progress, starts [_drainQueue] immediately.
  /// The input can be called at any time, including while a response is
  /// streaming — the message is held in [ChatState.pendingQueue] until the
  /// current response completes.
  Future<void> sendMessage(String message) async {
    if (message.trim().isEmpty) return;

    final api = _api;
    if (api == null) return;

    final current = state.value ?? const ChatState();
    String? conversationId = current.conversationId;

    // Create a new conversation if needed (once per chat session).
    if (conversationId == null) {
      try {
        final response = await api.conversations.createConversation(
          createConversationRequest: CreateConversationRequest((b) => b),
        );
        final conv = response.data!;
        conversationId = conv.id;
        ref.read(conversationListProvider.notifier).prependConversation(conv);
        state = AsyncData(
          (state.value ?? const ChatState()).copyWith(
            conversationId: conversationId,
          ),
        );
      } catch (e) {
        state = AsyncData(
          current.copyWith(error: 'Failed to create conversation: $e'),
        );
        return;
      }
    }

    // Append message to pending queue, capturing conversationId at enqueue time.
    final afterCreate = state.value ?? const ChatState();
    state = AsyncData(
      afterCreate.copyWith(
        pendingQueue: [
          ...afterCreate.pendingQueue,
          PendingMessage(text: message, conversationId: conversationId),
        ],
      ),
    );

    // Kick off drain if not already running.
    if (!_draining) {
      _drainQueue();
    }
  }

  /// Upload [audioBytes] to the voice endpoint and stream the response.
  ///
  /// Creates a conversation if none is active (same as [sendMessage]).
  Future<void> sendVoiceMessage(Uint8List audioBytes, String mimeType) async {
    final api = _api;
    if (api == null) return;

    final current = state.value ?? const ChatState();
    String? conversationId = current.conversationId;

    if (conversationId == null) {
      try {
        final response = await api.conversations.createConversation(
          createConversationRequest: CreateConversationRequest((b) => b),
        );
        final conv = response.data!;
        conversationId = conv.id;
        ref.read(conversationListProvider.notifier).prependConversation(conv);
        state = AsyncData(
          (state.value ?? const ChatState()).copyWith(
            conversationId: conversationId,
          ),
        );
      } catch (e) {
        state = AsyncData(
          current.copyWith(error: 'Failed to create conversation: $e'),
        );
        return;
      }
    }

    await _streamVoiceMessage(audioBytes, mimeType, conversationId);
  }

  /// Retry a failed message.
  ///
  /// Removes the failed [msg] from the message list and re-enqueues its
  /// content through [sendMessage].
  Future<void> retryMessage(ChatMessage msg) async {
    final current = state.value ?? const ChatState();
    final msgs = List<ChatMessage>.from(current.messages)
      ..removeWhere((m) => m.id == msg.id);
    state = AsyncData(current.copyWith(messages: msgs));
    await sendMessage(msg.content);
  }

  /// Pop messages from [ChatState.pendingQueue] and stream them one at a time.
  ///
  /// A [_draining] guard prevents re-entrant calls. The loop continues until
  /// the queue is empty; try/finally ensures the flag is always cleared.
  Future<void> _drainQueue() async {
    if (_draining) return;
    _draining = true;
    try {
      while (ref.mounted) {
        final current = state.value ?? const ChatState();
        if (current.pendingQueue.isEmpty) break;

        final pending = current.pendingQueue.first;
        state = AsyncData(
          current.copyWith(pendingQueue: current.pendingQueue.sublist(1)),
        );

        await _streamMessage(pending.text, pending.conversationId);
      }
    } finally {
      _draining = false;
    }
  }

  /// Upload voice audio and stream the assistant's SSE response.
  Future<void> _streamVoiceMessage(
    Uint8List audioBytes,
    String mimeType,
    String conversationId,
  ) async {
    final api = _api;
    if (api == null) return;

    _cancelled = false;
    final current = state.value ?? const ChatState();

    // Show a placeholder user message while transcribing.
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    final userMsg = ChatMessage(
      id: userMsgId,
      role: 'user',
      content: '🎤 Voice message',
      status: MessageStatus.sending,
    );
    final assistantPlaceholder = ChatMessage(
      id: 'assistant-streaming',
      role: 'assistant',
      content: '',
      isStreaming: true,
    );
    state = AsyncData(
      current.copyWith(
        messages: [...current.messages, userMsg, assistantPlaceholder],
        isSending: true,
        streamingContent: '',
      ),
    );

    try {
      await for (final event in api.sendVoiceMessage(
        conversationId,
        audioBytes,
        mimeType,
      )) {
        if (_cancelled) break;
        final chatState = state.value ?? const ChatState();

        if (event is TokenEvent) {
          final newContent = chatState.streamingContent + event.token;
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) msgs[idx].content = newContent;
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              streamingContent: newContent,
              clearStatusMessage: true,
            ),
          );
        } else if (event is StatusEvent) {
          state = AsyncData(chatState.copyWith(statusMessage: event.message));
        } else if (event is AudioReadyEvent) {
          // Store audioId on the streaming assistant message for auto-play.
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = msgs[idx].copyWith(audioId: event.audioId);
          }
          state = AsyncData(chatState.copyWith(messages: msgs));
        } else if (event is DoneEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages);
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(
              content: event.content.isNotEmpty
                  ? '[Voice] ${event.content}'
                  : '🎤 Voice message',
              status: MessageStatus.ok,
            );
          }
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = ChatMessage(
              id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
              role: 'assistant',
              content: event.content,
              audioId: msgs[idx].audioId,
            );
          }
          state = AsyncData(
            ChatState(
              conversationId: conversationId,
              messages: msgs,
              pendingQueue: chatState.pendingQueue,
            ),
          );
          ref.read(conversationListProvider.notifier).refresh();
          return;
        } else if (event is ErrorEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages)
            ..removeWhere((m) => m.id == 'assistant-streaming');
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(
              status: MessageStatus.failed,
            );
          }
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              isSending: false,
              streamingContent: '',
              error: event.message,
            ),
          );
          return;
        }
      }
    } catch (e) {
      final chatState = state.value ?? const ChatState();
      final msgs = List<ChatMessage>.from(chatState.messages)
        ..removeWhere((m) => m.id == 'assistant-streaming');
      final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
      if (userIdx != -1) {
        msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.failed);
      }
      state = AsyncData(
        chatState.copyWith(
          messages: msgs,
          isSending: false,
          streamingContent: '',
          error: 'Voice stream error: $e',
        ),
      );
    }
  }

  /// Stream a single [message] to the server and update state from SSE events.
  Future<void> _streamMessage(String message, String conversationId) async {
    final api = _api;
    if (api == null) return;

    _cancelled = false;
    final current = state.value ?? const ChatState();

    // Add user message with status=sending and assistant streaming placeholder.
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    final userMsg = ChatMessage(
      id: userMsgId,
      role: 'user',
      content: message,
      status: MessageStatus.sending,
    );
    final assistantPlaceholder = ChatMessage(
      id: 'assistant-streaming',
      role: 'assistant',
      content: '',
      isStreaming: true,
    );

    state = AsyncData(
      current.copyWith(
        messages: [...current.messages, userMsg, assistantPlaceholder],
        isSending: true,
        streamingContent: '',
      ),
    );

    // Stream SSE events.
    try {
      await for (final event in api.streamMessages(conversationId, message)) {
        if (_cancelled) break;
        final chatState = state.value ?? const ChatState();

        if (event is TokenEvent) {
          final newContent = chatState.streamingContent + event.token;
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx].content = newContent;
          }
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              streamingContent: newContent,
              clearStatusMessage: true,
            ),
          );
        } else if (event is StatusEvent) {
          state = AsyncData(chatState.copyWith(statusMessage: event.message));
        } else if (event is ToolResultEvent) {
          state = AsyncData(
            chatState.copyWith(
              clearStatusMessage: true,
              lastToolResult: ChatToolResult(
                toolName: event.toolName,
                status: event.status,
              ),
            ),
          );
        } else if (event is DoneEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages);
          // Mark user message as successfully acknowledged.
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.ok);
          }
          // Replace streaming placeholder with final assistant message.
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = ChatMessage(
              id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
              role: 'assistant',
              content: event.content,
              audioId: msgs[idx].audioId,
            );
          }
          state = AsyncData(
            ChatState(
              conversationId: conversationId,
              messages: msgs,
              pendingQueue: chatState.pendingQueue,
            ),
          );
          // Refresh conversation list to update timestamps/titles.
          ref.read(conversationListProvider.notifier).refresh();
          return;
        } else if (event is AudioReadyEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = msgs[idx].copyWith(audioId: event.audioId);
          }
          state = AsyncData(chatState.copyWith(messages: msgs));
        } else if (event is ErrorEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages)
            ..removeWhere((m) => m.id == 'assistant-streaming');
          // Mark user message as failed (keep it in the list for retry).
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(
              status: MessageStatus.failed,
            );
          }
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              isSending: false,
              streamingContent: '',
              error: event.message,
            ),
          );
          return;
        }
      }

      // Stream ended without DoneEvent — treat accumulated buffer as final.
      final finalState = state.value ?? const ChatState();
      if (finalState.isSending) {
        final msgs = List<ChatMessage>.from(finalState.messages);
        final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
        if (idx != -1) {
          msgs[idx] = ChatMessage(
            id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
            role: 'assistant',
            content: finalState.streamingContent.isNotEmpty
                ? finalState.streamingContent
                : '(incomplete response)',
          );
        }
        // Mark user message as ok — we got at least a partial response.
        final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
        if (userIdx != -1) {
          msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.ok);
        }
        state = AsyncData(
          ChatState(
            conversationId: conversationId,
            messages: msgs,
            pendingQueue: finalState.pendingQueue,
            error: finalState.streamingContent.isEmpty
                ? 'Response may be incomplete'
                : null,
          ),
        );
      }
    } catch (e) {
      final chatState = state.value ?? const ChatState();
      final msgs = List<ChatMessage>.from(chatState.messages)
        ..removeWhere((m) => m.id == 'assistant-streaming');
      // Mark user message as failed.
      final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
      if (userIdx != -1) {
        msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.failed);
      }
      state = AsyncData(
        chatState.copyWith(
          messages: msgs,
          isSending: false,
          streamingContent: '',
          error: 'Stream error: $e',
        ),
      );
    }
  }
}

/// Provider for [ChatNotifier].
final chatProvider = AsyncNotifierProvider.autoDispose<ChatNotifier, ChatState>(
  ChatNotifier.new,
);
