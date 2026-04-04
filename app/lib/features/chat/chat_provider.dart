import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/client.dart';
import '../../api/endpoints/conversations.dart';
import '../../api/models/conversation.dart';
import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';

// -- Client provider ---------------------------------------------------------

/// Creates an [AssistantClient] from the active [ServerProfile].
/// Returns `null` when not connected.
final assistantClientProvider = Provider<AssistantClient?>((ref) {
  final profile = ref.watch(activeProfileProvider);
  if (profile == null) return null;
  return AssistantClient(baseUrl: profile.baseUrl, token: profile.token);
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
    extends AutoDisposeAsyncNotifier<ConversationListState> {
  @override
  Future<ConversationListState> build() async {
    return _fetchAll();
  }

  ConversationsEndpoint? get _endpoint {
    final client = ref.read(assistantClientProvider);
    if (client == null) return null;
    return ConversationsEndpoint(client);
  }

  Future<ConversationListState> _fetchAll() async {
    final endpoint = _endpoint;
    if (endpoint == null) return const ConversationListState();

    try {
      final conversations = await endpoint.list();
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
    final endpoint = _endpoint;
    if (endpoint == null) return null;

    try {
      final created = await endpoint.create(title: title);
      final current = state.valueOrNull ?? const ConversationListState();
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

  /// Delete a conversation by ID.
  Future<void> deleteConversation(String id) async {
    final endpoint = _endpoint;
    if (endpoint == null) return;

    try {
      await endpoint.delete(id);
      final current = state.valueOrNull ?? const ConversationListState();
      state = AsyncData(
        current.copyWith(
          conversations:
              current.conversations.where((c) => c.id != id).toList(),
        ),
      );
    } catch (_) {
      // Silently ignore delete errors — the list will refresh on next load.
    }
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
class ChatNotifier extends AutoDisposeAsyncNotifier<ChatState> {
  @override
  Future<ChatState> build() async {
    return const ChatState();
  }

  ConversationsEndpoint? get _endpoint {
    final client = ref.read(assistantClientProvider);
    if (client == null) return null;
    return ConversationsEndpoint(client);
  }

  /// Load an existing conversation by ID.
  Future<void> loadConversation(String conversationId) async {
    state = AsyncData(
      (state.valueOrNull ?? const ChatState()).copyWith(
        isLoadingHistory: true,
        clearError: true,
      ),
    );

    final endpoint = _endpoint;
    if (endpoint == null) {
      state = AsyncData(
        const ChatState(error: 'Not connected to server'),
      );
      return;
    }

    try {
      final detail = await endpoint.get(conversationId);
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

  /// Set the active conversation ID without loading history.
  void setConversationId(String id) {
    state = AsyncData(
      (state.valueOrNull ?? const ChatState()).copyWith(conversationId: id),
    );
  }

  /// Send a user message and stream the assistant's response.
  Future<void> sendMessage(String message) async {
    if (message.trim().isEmpty) return;

    final endpoint = _endpoint;
    if (endpoint == null) return;

    final current = state.valueOrNull ?? const ChatState();
    String? conversationId = current.conversationId;

    // Create a new conversation if needed.
    if (conversationId == null) {
      try {
        final conv = await endpoint.create();
        conversationId = conv.id;
        // Also update the conversation list.
        ref
            .read(conversationListProvider.notifier)
            .createConversation(title: conv.title);
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
      await for (final event
          in endpoint.sendMessage(conversationId, message)) {
        final chatState = state.valueOrNull ?? const ChatState();

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
      final finalState = state.valueOrNull ?? const ChatState();
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
      final chatState = state.valueOrNull ?? const ChatState();
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
