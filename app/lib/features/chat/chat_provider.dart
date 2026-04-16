import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/capabilities_provider.dart';
import '../../api/models/server_capabilities.dart';
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

/// Status of a single tool call within an assistant message.
enum ToolCallStatus { pending, ok, error, denied }

/// A single tool invocation recorded on an assistant message.
class ToolCallRecord {
  ToolCallRecord({
    required this.toolName,
    required this.status,
    this.arguments,
    this.result,
  });

  final String toolName;

  /// Mutable so the pending chip can be updated to a resolved status in place
  /// during streaming, matching the pattern used for [ChatMessage.content].
  ToolCallStatus status;

  /// The JSON arguments passed to the tool (may be null while pending).
  Map<String, dynamic>? arguments;

  /// The tool's output, truncated for display (populated on completion).
  String? result;
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
    this.ttsAvailable = false,
    List<ToolCallRecord>? toolCalls,
  }) : toolCalls = toolCalls ?? [];

  final String id;
  final String role;
  String content;
  bool isStreaming;
  MessageStatus status;

  /// Non-null when the server has produced audio for this assistant message.
  final String? audioId;

  /// Whether TTS audio can be synthesised for this message.
  /// Set from history (`MessageSummary.ttsAvailable`) and updated during
  /// streaming when an [AudioReadyEvent] is received or the stream completes
  /// with content.
  bool ttsAvailable;

  /// Tool calls associated with this assistant message, in invocation order.
  /// Populated during streaming from [StatusEvent] and [ToolResultEvent].
  List<ToolCallRecord> toolCalls;

  bool get isUser => role == 'user';
  bool get isAssistant => role == 'assistant';

  ChatMessage copyWith({
    String? content,
    bool? isStreaming,
    MessageStatus? status,
    String? audioId,
    bool? ttsAvailable,
    List<ToolCallRecord>? toolCalls,
  }) {
    return ChatMessage(
      id: id,
      role: role,
      content: content ?? this.content,
      isStreaming: isStreaming ?? this.isStreaming,
      status: status ?? this.status,
      audioId: audioId ?? this.audioId,
      ttsAvailable: ttsAvailable ?? this.ttsAvailable,
      toolCalls: toolCalls ?? this.toolCalls,
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

  /// Extract a tool name from a [StatusEvent] message.
  ///
  /// The server emits messages like "Calling tool: web-search". Strip the
  /// prefix; fall back to the full string if the format doesn't match.
  static String _extractToolName(String statusMessage) {
    const prefix = 'Calling tool: ';
    if (statusMessage.startsWith(prefix)) {
      return statusMessage.substring(prefix.length).trim();
    }
    return statusMessage;
  }

  /// Map a [ToolResultEvent.status] string to a [ToolCallStatus] value.
  static ToolCallStatus _parseToolStatus(String status) {
    return switch (status) {
      'ok' => ToolCallStatus.ok,
      'error' => ToolCallStatus.error,
      'denied' => ToolCallStatus.denied,
      _ => ToolCallStatus.ok,
    };
  }

  /// Push a pending [ToolCallRecord] onto the streaming assistant message and
  /// update state.  Called from both [_streamMessage] and [_streamVoiceMessage]
  /// when a [StatusEvent] is received.
  void _onStatusEvent(ChatState chatState, String statusMessage) {
    final toolName = _extractToolName(statusMessage);
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
    if (idx != -1) {
      msgs[idx].toolCalls.add(
        ToolCallRecord(toolName: toolName, status: ToolCallStatus.pending),
      );
    }
    state = AsyncData(
      chatState.copyWith(messages: msgs, statusMessage: statusMessage),
    );
  }

  /// Resolve the pending [ToolCallRecord] for [event] and update state.
  /// Called from both [_streamMessage] and [_streamVoiceMessage].
  void _onToolResultEvent(ChatState chatState, ToolResultEvent event) {
    final resolvedStatus = _parseToolStatus(event.status);
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
    if (idx != -1) {
      final callIdx = msgs[idx].toolCalls.indexWhere(
        (tc) =>
            tc.toolName == event.toolName &&
            tc.status == ToolCallStatus.pending,
      );
      if (callIdx != -1) {
        msgs[idx].toolCalls[callIdx].status = resolvedStatus;
        msgs[idx].toolCalls[callIdx].arguments = event.arguments;
        msgs[idx].toolCalls[callIdx].result = event.result;
      } else {
        // No matching pending chip — append a resolved record directly.
        msgs[idx].toolCalls.add(
          ToolCallRecord(
            toolName: event.toolName,
            status: resolvedStatus,
            arguments: event.arguments,
            result: event.result,
          ),
        );
      }
    }
    state = AsyncData(
      chatState.copyWith(
        messages: msgs,
        clearStatusMessage: true,
        lastToolResult: ChatToolResult(
          toolName: event.toolName,
          status: event.status,
        ),
      ),
    );
  }

  /// Run ID of the most recent (or current) orchestrator run.
  /// Captured from the `RunStartedEvent` so we can reconnect if the stream drops.
  String? _currentRunId;

  /// Sequence number of the last event received from the current run.
  /// Used as the `since` cursor when requesting event-log replay.
  int _lastSeq = 0;

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
          .map(
            (m) => ChatMessage(
              id: m.id,
              role: m.role,
              content: m.content,
              ttsAvailable: m.ttsAvailable,
              toolCalls: m.toolCalls
                  ?.map(
                    (tc) => ToolCallRecord(
                      toolName: tc.name,
                      status: _parseToolStatus(tc.status),
                      arguments: tc.arguments?.asMap.cast<String, dynamic>(),
                      result: tc.result,
                    ),
                  )
                  .toList(),
            ),
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
  /// If a [_currentRunId] is available, attempts to reconnect via the event-log
  /// replay endpoint before re-sending the message.  Falls back to re-sending
  /// on 404 (unknown run) or 410 (events pruned).
  Future<void> retryMessage(ChatMessage msg) async {
    final api = _api;
    final current = state.value ?? const ChatState();
    final conversationId = current.conversationId;

    if (api != null && _currentRunId != null && conversationId != null) {
      // Attempt replay before re-sending.
      final replayed = await _replayRun(
        api,
        conversationId,
        msg.id,
        _currentRunId!,
        _lastSeq,
      );
      if (replayed) return;
    }

    // Fallback: remove the failed message and re-enqueue it.
    final msgs = List<ChatMessage>.from(current.messages)
      ..removeWhere((m) => m.id == msg.id);
    state = AsyncData(current.copyWith(messages: msgs));
    await sendMessage(msg.content);
  }

  /// Attempt to resume streaming via the event-log replay endpoint.
  ///
  /// Returns `true` if the run completed successfully via replay, `false` if
  /// the replay endpoint returned 404/410 or another error (caller should
  /// fall back to re-sending).
  Future<bool> _replayRun(
    ApiClient api,
    String conversationId,
    String userMsgId,
    String runId,
    int since,
  ) async {
    final current = state.value ?? const ChatState();

    // Put the user message back into sending state and add streaming placeholder.
    final msgs = List<ChatMessage>.from(current.messages);
    final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
    if (userIdx != -1) {
      msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.sending);
    }
    final placeholder = ChatMessage(
      id: 'assistant-streaming',
      role: 'assistant',
      content: '',
      isStreaming: true,
    );
    state = AsyncData(
      current.copyWith(
        messages: [...msgs, placeholder],
        isSending: true,
        streamingContent: '',
      ),
    );

    try {
      await for (final event in api.streamEventsFrom(
        conversationId,
        runId,
        since: since,
      )) {
        if (_cancelled) break;
        final chatState = state.value ?? const ChatState();

        if (event is RunStartedEvent) {
          _lastSeq++;
        } else if (event is TokenEvent) {
          _lastSeq++;
          final newContent = chatState.streamingContent + event.token;
          final ms = List<ChatMessage>.from(chatState.messages);
          final idx = ms.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) ms[idx].content = newContent;
          state = AsyncData(
            chatState.copyWith(
              messages: ms,
              streamingContent: newContent,
              clearStatusMessage: true,
            ),
          );
        } else if (event is StatusEvent) {
          _lastSeq++;
          _onStatusEvent(chatState, event.message);
        } else if (event is ToolResultEvent) {
          _lastSeq++;
          _onToolResultEvent(chatState, event);
        } else if (event is DoneEvent) {
          _lastSeq++;
          final ms = List<ChatMessage>.from(chatState.messages);
          final uIdx = ms.indexWhere((m) => m.id == userMsgId);
          if (uIdx != -1) {
            ms[uIdx] = ms[uIdx].copyWith(status: MessageStatus.ok);
          }
          final aIdx = ms.indexWhere((m) => m.id == 'assistant-streaming');
          if (aIdx != -1) {
            final placeholder = ms[aIdx];
            ms[aIdx] = ChatMessage(
              id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
              role: 'assistant',
              content: event.content,
              toolCalls: placeholder.toolCalls,
            );
          }
          state = AsyncData(
            ChatState(
              conversationId: conversationId,
              messages: ms,
              pendingQueue: chatState.pendingQueue,
            ),
          );
          ref.read(conversationListProvider.notifier).refresh();
          return true;
        } else if (event is ErrorEvent) {
          _lastSeq++;
          return false;
        }
      }
      return false;
    } on DioException catch (e) {
      // 404 = run not found, 410 = run expired → caller re-sends.
      final status = e.response?.statusCode;
      if (status == 404 || status == 410) {
        // Restore UI to clean failed state.
        final chatState = state.value ?? const ChatState();
        final ms = List<ChatMessage>.from(chatState.messages)
          ..removeWhere((m) => m.id == 'assistant-streaming');
        final uIdx = ms.indexWhere((m) => m.id == userMsgId);
        if (uIdx != -1) {
          ms[uIdx] = ms[uIdx].copyWith(status: MessageStatus.failed);
        }
        state = AsyncData(
          chatState.copyWith(
            messages: ms,
            isSending: false,
            streamingContent: '',
          ),
        );
        return false;
      }
      return false;
    } catch (_) {
      return false;
    }
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

        if (event is TranscriptEvent) {
          // Update the user bubble with the actual spoken text.
          final msgs = List<ChatMessage>.from(chatState.messages);
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(
              content: event.transcript,
              status: MessageStatus.ok,
            );
          }
          state = AsyncData(chatState.copyWith(messages: msgs));
        } else if (event is TokenEvent) {
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
          _onStatusEvent(chatState, event.message);
        } else if (event is ToolResultEvent) {
          _onToolResultEvent(chatState, event);
        } else if (event is AudioReadyEvent) {
          // Store audioId on the streaming assistant message for auto-play.
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = msgs[idx].copyWith(
              audioId: event.audioId,
              ttsAvailable: true,
            );
          }
          state = AsyncData(chatState.copyWith(messages: msgs));
        } else if (event is DoneEvent) {
          final msgs = List<ChatMessage>.from(chatState.messages);
          // User bubble is already correct from TranscriptEvent; just ensure ok status.
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.ok);
          }
          // Preserve tool call records and audio id on the final message.
          final assistantId =
              event.messageId ??
              'assistant-${DateTime.now().millisecondsSinceEpoch}';
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            final placeholder = msgs[idx];
            final caps =
                ref.read(capabilitiesProvider).value ??
                ServerCapabilities.disabled;
            final ttsAvail =
                placeholder.ttsAvailable ||
                (caps.voiceReceive && event.content.isNotEmpty);
            msgs[idx] = ChatMessage(
              id: assistantId,
              role: 'assistant',
              content: event.content,
              audioId: placeholder.audioId,
              ttsAvailable: ttsAvail,
              toolCalls: placeholder.toolCalls,
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
    _currentRunId = null;
    _lastSeq = 0;
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

        if (event is RunStartedEvent) {
          // Capture the run ID for potential reconnect; don't change UI.
          _currentRunId = event.runId;
          _lastSeq++;
          continue;
        } else if (event is TokenEvent) {
          _lastSeq++;
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
          _lastSeq++;
          _onStatusEvent(chatState, event.message);
        } else if (event is ToolResultEvent) {
          _lastSeq++;
          _onToolResultEvent(chatState, event);
        } else if (event is DoneEvent) {
          _lastSeq++;
          final msgs = List<ChatMessage>.from(chatState.messages);
          // Mark user message as successfully acknowledged.
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.ok);
          }
          // Replace streaming placeholder with final assistant message,
          // preserving audio id and accumulated tool call records.
          final assistantId =
              event.messageId ??
              'assistant-${DateTime.now().millisecondsSinceEpoch}';
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            final placeholder = msgs[idx];
            final caps =
                ref.read(capabilitiesProvider).value ??
                ServerCapabilities.disabled;
            final ttsAvail =
                placeholder.ttsAvailable ||
                (caps.voiceReceive && event.content.isNotEmpty);
            msgs[idx] = ChatMessage(
              id: assistantId,
              role: 'assistant',
              content: event.content,
              audioId: placeholder.audioId,
              ttsAvailable: ttsAvail,
              toolCalls: placeholder.toolCalls,
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
          _lastSeq++;
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            msgs[idx] = msgs[idx].copyWith(
              audioId: event.audioId,
              ttsAvailable: true,
            );
          }
          state = AsyncData(chatState.copyWith(messages: msgs));
        } else if (event is ErrorEvent) {
          _lastSeq++;
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
          final placeholder = msgs[idx];
          msgs[idx] = ChatMessage(
            id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
            role: 'assistant',
            content: finalState.streamingContent.isNotEmpty
                ? finalState.streamingContent
                : '(incomplete response)',
            toolCalls: placeholder.toolCalls,
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
      // If we have a run ID, attempt replay before marking as failed.
      if (_currentRunId != null) {
        final replayed = await _replayRun(
          api,
          conversationId,
          userMsgId,
          _currentRunId!,
          _lastSeq,
        );
        if (replayed) return;
      }

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
