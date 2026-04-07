import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';

// -- Client provider ---------------------------------------------------------

/// Creates an [ApiClient] from the active [ServerProfile].
/// Returns `null` when not connected.
final apiClientProvider = Provider<ApiClient?>((ref) {
  final profile = ref.watch(activeProfileProvider);
  if (profile == null) return null;
  return ApiClient(baseUrl: profile.baseUrl, token: profile.token);
});

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
class ConversationListNotifier
    extends AsyncNotifier<ConversationListState> {
  @override
  Future<ConversationListState> build() async {
    return _fetchAll();
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  Future<ConversationListState> _fetchAll() async {
    final api = _api;
    if (api == null) return const ConversationListState();

    try {
      final response = await api.conversations.listConversations();
      final conversations = response.data!.toList();
      return ConversationListState(conversations: conversations);
    } catch (e) {
      return ConversationListState(error: e.toString());
    }
  }

  /// Reload conversations from the server.
  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(await _fetchAll());
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
        current.copyWith(
          conversations: [created, ...current.conversations],
        ),
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
      current.copyWith(
        conversations: [conv, ...current.conversations],
      ),
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
final conversationListProvider = AsyncNotifierProvider.autoDispose<
    ConversationListNotifier, ConversationListState>(
  ConversationListNotifier.new,
);

// -- Active chat state -------------------------------------------------------

/// A message shown in the chat UI (may be a streaming partial).
class ChatMessage {
  ChatMessage({
    required this.id,
    required this.role,
    required this.content,
    this.isStreaming = false,
  });

  final String id;
  final String role;
  String content;
  bool isStreaming;

  bool get isUser => role == 'user';
  bool get isAssistant => role == 'assistant';
}

/// State for the active chat.
class ChatState {
  const ChatState({
    this.conversationId,
    this.messages = const [],
    this.isSending = false,
    this.isLoadingHistory = false,
    this.streamingContent = '',
    this.error,
  });

  final String? conversationId;
  final List<ChatMessage> messages;
  final bool isSending;
  final bool isLoadingHistory;

  /// Accumulated token content during streaming (before DoneEvent).
  final String streamingContent;
  final String? error;

  bool get isStreaming => isSending && streamingContent.isNotEmpty;

  ChatState copyWith({
    String? conversationId,
    List<ChatMessage>? messages,
    bool? isSending,
    bool? isLoadingHistory,
    String? streamingContent,
    String? error,
    bool clearError = false,
    bool clearConversation = false,
  }) {
    return ChatState(
      conversationId:
          clearConversation ? null : (conversationId ?? this.conversationId),
      messages: messages ?? this.messages,
      isSending: isSending ?? this.isSending,
      isLoadingHistory: isLoadingHistory ?? this.isLoadingHistory,
      streamingContent: streamingContent ?? this.streamingContent,
      error: clearError ? null : (error ?? this.error),
    );
  }
}

/// Manages the active chat conversation.
class ChatNotifier extends AsyncNotifier<ChatState> {
  @override
  Future<ChatState> build() async {
    return const ChatState();
  }

  ApiClient? get _api => ref.read(apiClientProvider);

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
      state = AsyncData(
        const ChatState(error: 'Not connected to server'),
      );
      return;
    }

    try {
      final response = await api.conversations.getConversation(id: conversationId);
      final detail = response.data!;
      final messages = detail.messages
          .map(
            (m) => ChatMessage(id: m.id, role: m.role, content: m.content),
          )
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

  /// Send a user message and stream the assistant's response.
  Future<void> sendMessage(String message) async {
    if (message.trim().isEmpty) return;

    final api = _api;
    if (api == null) return;

    final current = state.value ?? const ChatState();
    String? conversationId = current.conversationId;

    // Create a new conversation if needed.
    if (conversationId == null) {
      try {
        final response = await api.conversations.createConversation(
          createConversationRequest: CreateConversationRequest((b) => b),
        );
        final conv = response.data!;
        conversationId = conv.id;
        // Reflect the new conversation in the local list (no extra POST).
        ref.read(conversationListProvider.notifier).prependConversation(conv);
      } catch (e) {
        state = AsyncData(
          current.copyWith(error: 'Failed to create conversation: $e'),
        );
        return;
      }
    }

    // Add the user message immediately.
    final userMsg = ChatMessage(
      id: 'user-${DateTime.now().millisecondsSinceEpoch}',
      role: 'user',
      content: message,
    );

    // Add a streaming placeholder for the assistant.
    final assistantPlaceholder = ChatMessage(
      id: 'assistant-streaming',
      role: 'assistant',
      content: '',
      isStreaming: true,
    );

    state = AsyncData(
      ChatState(
        conversationId: conversationId,
        messages: [...current.messages, userMsg, assistantPlaceholder],
        isSending: true,
        streamingContent: '',
      ),
    );

    // Stream SSE events.
    try {
      await for (final event in api.streamMessages(conversationId, message)) {
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
            ),
          );
        } else if (event is DoneEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = ChatMessage(
              id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
              role: 'assistant',
              content: event.content,
            );
          }
          state = AsyncData(
            ChatState(
              conversationId: conversationId,
              messages: msgs,
            ),
          );
          // Refresh conversation list to update timestamps/titles.
          ref.read(conversationListProvider.notifier).refresh();
          return;
        } else if (event is ErrorEvent) {
          // Remove streaming placeholder, show error.
          final msgs = List<ChatMessage>.from(chatState.messages)
            ..removeWhere((m) => m.id == 'assistant-streaming');
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
        state = AsyncData(
          ChatState(
            conversationId: conversationId,
            messages: msgs,
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
final chatProvider =
    AsyncNotifierProvider.autoDispose<ChatNotifier, ChatState>(
  ChatNotifier.new,
);
