import 'dart:async';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:dio/dio.dart';
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

  final List<ConversationListEntry> conversations;
  final bool isLoading;
  final String? error;

  ConversationListState copyWith({
    List<ConversationListEntry>? conversations,
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

/// Manages the list of conversations via an SSE stream.
///
/// Subscribes to `GET /api/conversations/stream` which sends an initial
/// `snapshot` event followed by `upserted`/`deleted` deltas. Local state
/// is patched reactively — no manual refresh needed.
class ConversationListNotifier extends AsyncNotifier<ConversationListState> {
  static const _debounceDuration = Duration(milliseconds: 300);

  StreamSubscription<ConversationListEvent>? _subscription;
  Timer? _debounceTimer;
  final Map<String, ConversationListEntry> _pendingUpserts = {};

  @override
  Future<ConversationListState> build() async {
    final api = ref.watch(apiClientProvider);
    if (api == null) {
      _resetStream();
      return const ConversationListState();
    }

    _subscribe(api);

    ref.onDispose(() {
      _subscription?.cancel();
      _subscription = null;
      _debounceTimer?.cancel();
      _debounceTimer = null;
    });

    // Return loading state — the snapshot event will replace it.
    return const ConversationListState(isLoading: true);
  }

  void _resetStream() {
    _subscription?.cancel();
    _subscription = null;
    _debounceTimer?.cancel();
    _debounceTimer = null;
    _pendingUpserts.clear();
  }

  void _subscribe(ApiClient api) {
    _resetStream();
    _subscription = api.streamConversations().listen(
      _onEvent,
      onError: (Object e) {
        final message = e is ApiAuthException
            ? e.message
            : 'Connection error: $e';
        state = AsyncData(ConversationListState(error: message));
      },
      cancelOnError: false,
    );
  }

  void _onEvent(ConversationListEvent event) {
    switch (event) {
      case ConversationSnapshotEvent(:final conversations):
        _flushPendingUpserts();
        state = AsyncData(ConversationListState(conversations: conversations));
      case ConversationUpsertedEvent(:final conversation):
        _pendingUpserts[conversation.id] = conversation;
        _debounceTimer?.cancel();
        _debounceTimer = Timer(_debounceDuration, _flushPendingUpserts);
      case ConversationDeletedEvent(:final conversationId):
        _pendingUpserts.remove(conversationId);
        _remove(conversationId);
    }
  }

  void _flushPendingUpserts() {
    if (_pendingUpserts.isEmpty) return;
    _debounceTimer?.cancel();
    _debounceTimer = null;

    final current = state.value ?? const ConversationListState();
    final ids = _pendingUpserts.keys.toSet();
    final updated =
        current.conversations.where((c) => !ids.contains(c.id)).toList()
          ..addAll(_pendingUpserts.values)
          ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    _pendingUpserts.clear();
    state = AsyncData(current.copyWith(conversations: updated));
  }

  void _upsert(ConversationListEntry conv) {
    final current = state.value ?? const ConversationListState();
    final updated = current.conversations.where((c) => c.id != conv.id).toList()
      ..insert(0, conv)
      ..sort((a, b) => b.updatedAt.compareTo(a.updatedAt));
    state = AsyncData(current.copyWith(conversations: updated));
  }

  void _remove(String id) {
    final current = state.value ?? const ConversationListState();
    state = AsyncData(
      current.copyWith(
        conversations: current.conversations.where((c) => c.id != id).toList(),
      ),
    );
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  /// Reconnect the stream. Useful as a retry after errors.
  Future<void> refresh() async {
    final api = _api;
    if (api == null) return;
    state = const AsyncLoading();
    _subscribe(api);
  }

  /// Create a new conversation. Returns the ID for navigation.
  /// The stream will deliver the list update automatically.
  Future<String?> createConversation({String? title}) async {
    final api = _api;
    if (api == null) return null;

    try {
      final response = await api.conversations.createConversation(
        createConversationRequest: CreateConversationRequest(
          (b) => b.title = title ?? 'New Chat',
        ),
      );
      return response.data?.id;
    } catch (e) {
      return null;
    }
  }

  /// Add an already-created conversation to the front of the local list.
  ///
  /// Accepts a [ConversationSummary] from the generated API and converts it.
  /// This is a compatibility shim — the stream will also deliver the update,
  /// but this provides immediate feedback before the event arrives.
  void prependConversation(ConversationSummary conv) {
    _upsert(
      ConversationListEntry(
        id: conv.id,
        title: conv.title,
        createdAt: conv.createdAt.toUtc(),
        updatedAt: conv.updatedAt.toUtc(),
      ),
    );
  }

  /// Delete a conversation by ID.
  /// The stream will deliver the list update automatically.
  Future<void> deleteConversation(String id) async {
    final api = _api;
    if (api == null) return;
    await api.conversations.deleteConversation(id: id);
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
    this.toolCallId,
    this.arguments,
    this.result,
    DateTime? startedAt,
    this.duration,
  }) : startedAt = startedAt ?? DateTime.now();

  final String toolName;

  /// Provider-assigned tool-call ID (e.g. Anthropic `tool_use_id`).
  /// Used to correlate StatusEvent → ToolResultEvent when available.
  final String? toolCallId;

  /// Mutable so the pending chip can be updated to a resolved status in place
  /// during streaming, matching the pattern used for [ChatMessage.content].
  ToolCallStatus status;

  /// The JSON arguments passed to the tool (may be null while pending).
  Map<String, dynamic>? arguments;

  /// The tool's output, truncated for display (populated on completion).
  String? result;

  /// When this tool call started (set on StatusEvent).
  final DateTime startedAt;

  /// Elapsed time from start to completion. Null while pending.
  Duration? duration;
}

/// The kind of entry shown in the chat timeline.
enum TimelineEntryType { message, thinking, toolCall, subagent, command }

/// A message shown in the chat UI (may be a streaming partial).
/// Metadata for an image attachment on a message.
class ChatAttachment {
  const ChatAttachment({
    required this.id,
    required this.filename,
    required this.mimeType,
    required this.url,
  });

  final String id;
  final String filename;
  final String mimeType;
  final String url;
}

class ChatMessage {
  ChatMessage({
    required this.id,
    required this.role,
    required this.content,
    this.isStreaming = false,
    this.status = MessageStatus.ok,
    this.audioId,
    this.ttsAvailable = false,
    this.tokenStream,
    this.audioBytes,
    this.audioMimeType,
    this.timelineType = TimelineEntryType.message,
    this.thinkingContent,
    this.subagentId,
    this.subagentTask,
    this.subagentSummary,
    this.commandName,
    this.commandAckText,
    List<ToolCallRecord>? toolCalls,
    List<ChatAttachment>? attachments,
  }) : toolCalls = toolCalls ?? [],
       attachments = attachments ?? [];

  final String id;
  final String role;
  String content;
  bool isStreaming;

  /// Whether this timeline entry has been superseded by newer activity
  /// (e.g. final answer streaming has begun). Used to derive [EntryState.stale].
  bool isStale = false;

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

  /// Live stream of token chunks during streaming. Used by [StreamMarkdown]
  /// for throttled incremental rendering. Null for non-streaming messages.
  Stream<String>? tokenStream;

  /// Image attachments linked to this message.
  List<ChatAttachment> attachments;

  /// Raw audio bytes for user voice messages (in-memory only, not persisted).
  final Uint8List? audioBytes;

  /// MIME type of [audioBytes] (e.g. `audio/webm`, `audio/mp4`).
  final String? audioMimeType;

  /// The kind of timeline entry this message represents.
  final TimelineEntryType timelineType;

  /// Reasoning text for [TimelineEntryType.thinking] entries.
  final String? thinkingContent;

  /// Live stream of thinking tokens during streaming. Used by the timeline
  /// entry for incremental rendering. Null for non-streaming messages.
  Stream<String>? thinkingTokenStream;

  /// Subagent identifier for [TimelineEntryType.subagent] entries.
  final String? subagentId;

  /// Subagent task description for [TimelineEntryType.subagent] entries.
  final String? subagentTask;

  /// Subagent completion summary for [TimelineEntryType.subagent] entries.
  String? subagentSummary;

  /// Accumulated content tokens produced by a subagent.
  String subagentContent = '';

  /// Accumulated thinking text produced by a subagent.
  String subagentThinking = '';

  /// Tool calls executed by a subagent, in order.
  List<ToolCallRecord> subagentToolCalls = [];

  /// Command name for [TimelineEntryType.command] entries (without `/`).
  final String? commandName;

  /// Acknowledgement text for [TimelineEntryType.command] entries.
  final String? commandAckText;

  bool get isUser => role == 'user';
  bool get isAssistant => role == 'assistant';

  ChatMessage copyWith({
    String? content,
    bool? isStreaming,
    MessageStatus? status,
    String? audioId,
    bool? ttsAvailable,
    List<ToolCallRecord>? toolCalls,
    Stream<String>? tokenStream,
    bool clearTokenStream = false,
    List<ChatAttachment>? attachments,
    Uint8List? audioBytes,
    String? audioMimeType,
    TimelineEntryType? timelineType,
    String? thinkingContent,
    String? subagentId,
    String? subagentTask,
    String? subagentSummary,
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
      tokenStream: clearTokenStream ? null : (tokenStream ?? this.tokenStream),
      attachments: attachments ?? this.attachments,
      audioBytes: audioBytes ?? this.audioBytes,
      audioMimeType: audioMimeType ?? this.audioMimeType,
      timelineType: timelineType ?? this.timelineType,
      thinkingContent: thinkingContent ?? this.thinkingContent,
      subagentId: subagentId ?? this.subagentId,
      subagentTask: subagentTask ?? this.subagentTask,
      subagentSummary: subagentSummary ?? this.subagentSummary,
    );
  }
}

/// A message waiting in the send queue.
///
/// Stores the message text together with the [conversationId] that was active
/// at enqueue time, so messages are always routed to the correct conversation
/// even if the user navigates away while the queue is draining.
class PendingMessage {
  const PendingMessage({
    required this.text,
    required this.conversationId,
    this.attachmentIds = const [],
    this.attachments = const [],
  });
  final String text;
  final String conversationId;
  final List<String> attachmentIds;

  /// Pre-built attachment metadata from upload responses, used to show
  /// thumbnails immediately in the user bubble without waiting for a reload.
  final List<ChatAttachment> attachments;
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
  StreamController<String>? _thinkingController;

  void _closeThinkingController() {
    final c = _thinkingController;
    _thinkingController = null;
    if (c != null) unawaited(c.close());
  }

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

  /// Mark all currently-active timeline entries (thinking, subagent) as
  /// complete. Called when a new timeline entry starts, so the previous one
  /// auto-collapses.
  void _completeActiveTimelineEntries(List<ChatMessage> msgs) {
    for (var i = 0; i < msgs.length; i++) {
      if (msgs[i].timelineType != TimelineEntryType.message &&
          msgs[i].isStreaming) {
        msgs[i].isStreaming = false;
      }
    }
  }

  /// Mark all timeline entries as stale — called when final answer tokens
  /// begin streaming, causing all timeline entries to collapse with reduced
  /// opacity.
  void _markTimelineEntriesStale(List<ChatMessage> msgs) {
    for (var i = 0; i < msgs.length; i++) {
      if (msgs[i].timelineType != TimelineEntryType.message) {
        msgs[i].isStreaming = false;
        msgs[i].isStale = true;
      }
    }
  }

  /// Create a tool-call timeline entry and insert it before the streaming
  /// assistant placeholder. Called from both [_streamMessage] and
  /// [_streamVoiceMessage] when a [StatusEvent] is received.
  void _onStatusEvent(ChatState chatState, StatusEvent event) {
    final toolName = _extractToolName(event.message);
    final msgs = List<ChatMessage>.from(chatState.messages);

    // Complete any currently-active timeline entries before adding a new one.
    _completeActiveTimelineEntries(msgs);

    final entryId =
        'toolcall-$toolName-${DateTime.now().millisecondsSinceEpoch}';
    final entry = ChatMessage(
      id: entryId,
      role: 'assistant',
      content: '',
      isStreaming: true,
      timelineType: TimelineEntryType.toolCall,
      toolCalls: [
        ToolCallRecord(
          toolName: toolName,
          status: ToolCallStatus.pending,
          toolCallId: event.toolCallId,
        ),
      ],
    );

    // Insert before the assistant-streaming placeholder so tool calls render
    // above the reply bubble.
    final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
    if (idx != -1) {
      msgs.insert(idx, entry);
    } else {
      msgs.add(entry);
    }

    state = AsyncData(
      chatState.copyWith(messages: msgs, statusMessage: event.message),
    );
  }

  /// Resolve the pending tool-call timeline entry for [event] and update state.
  /// Called from both [_streamMessage] and [_streamVoiceMessage].
  void _onToolResultEvent(ChatState chatState, ToolResultEvent event) {
    final resolvedStatus = _parseToolStatus(event.status);
    final msgs = List<ChatMessage>.from(chatState.messages);

    // Match by toolCallId when available (stable), fall back to tool name.
    final entryIdx = event.toolCallId != null
        ? msgs.lastIndexWhere(
            (m) =>
                m.timelineType == TimelineEntryType.toolCall &&
                m.toolCalls.isNotEmpty &&
                m.toolCalls.first.toolCallId == event.toolCallId &&
                m.toolCalls.first.status == ToolCallStatus.pending,
          )
        : msgs.lastIndexWhere(
            (m) =>
                m.timelineType == TimelineEntryType.toolCall &&
                m.toolCalls.isNotEmpty &&
                m.toolCalls.first.toolName == event.toolName &&
                m.toolCalls.first.status == ToolCallStatus.pending,
          );

    if (entryIdx != -1) {
      final record = msgs[entryIdx].toolCalls.first;
      record.status = resolvedStatus;
      record.arguments = event.arguments;
      record.result = event.result;
      record.duration = DateTime.now().difference(record.startedAt);
      msgs[entryIdx].isStreaming = false;
    } else {
      // No matching pending entry — insert a resolved one directly.
      final entryId =
          'toolcall-${event.toolName}-${DateTime.now().millisecondsSinceEpoch}';
      final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
      final entry = ChatMessage(
        id: entryId,
        role: 'assistant',
        content: '',
        timelineType: TimelineEntryType.toolCall,
        toolCalls: [
          ToolCallRecord(
            toolName: event.toolName,
            status: resolvedStatus,
            arguments: event.arguments,
            result: event.result,
          ),
        ],
      );
      if (idx != -1) {
        msgs.insert(idx, entry);
      } else {
        msgs.add(entry);
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

  /// Insert or update a thinking timeline entry in the message list.
  /// Multiple [ThinkingEvent]s accumulate into a single entry.
  /// Also feeds tokens to the thinking stream for incremental rendering.
  void _onThinkingEvent(ChatState chatState, ThinkingEvent event) {
    // Feed token to the thinking stream controller.
    _thinkingController?.add(event.content);

    final msgs = List<ChatMessage>.from(chatState.messages);
    final existing = msgs.indexWhere(
      (m) => m.timelineType == TimelineEntryType.thinking && m.isStreaming,
    );
    if (existing != -1) {
      // Accumulate into existing thinking entry.
      final prev = msgs[existing];
      msgs[existing] = prev.copyWith(
        thinkingContent: (prev.thinkingContent ?? '') + event.content,
      );
    } else {
      // Complete previous active timeline entries before starting a new one.
      _completeActiveTimelineEntries(msgs);
      // Insert a new thinking entry before the assistant-streaming placeholder.
      final insertIdx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
      final entry = ChatMessage(
        id: 'thinking-${DateTime.now().millisecondsSinceEpoch}',
        role: 'assistant',
        content: '',
        isStreaming: true,
        timelineType: TimelineEntryType.thinking,
        thinkingContent: event.content,
      );
      entry.thinkingTokenStream = _thinkingController?.stream;
      if (insertIdx != -1) {
        msgs.insert(insertIdx, entry);
      } else {
        msgs.add(entry);
      }
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Insert a subagent timeline entry when a subagent starts.
  void _onSubagentStartedEvent(
    ChatState chatState,
    SubagentStartedEvent event,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    final insertIdx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
    final entry = ChatMessage(
      id: 'subagent-${event.agentId}',
      role: 'assistant',
      content: '',
      timelineType: TimelineEntryType.subagent,
      subagentId: event.agentId,
      subagentTask: event.task,
    );
    if (insertIdx != -1) {
      msgs.insert(insertIdx, entry);
    } else {
      msgs.add(entry);
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Update the matching subagent entry with completion summary.
  void _onSubagentCompletedEvent(
    ChatState chatState,
    SubagentCompletedEvent event,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere(
      (m) =>
          m.timelineType == TimelineEntryType.subagent &&
          m.subagentId == event.agentId,
    );
    if (idx != -1) {
      msgs[idx].subagentSummary = event.summary;
      msgs[idx].isStreaming = false;
      // Mark cancelled subagents as stale for amber/collapsed display.
      if (event.status == 'cancelled') {
        msgs[idx].isStale = true;
      }
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Accumulate token content from a subagent into its timeline entry.
  void _onSubagentTokenEvent(ChatState chatState, SubagentTokenEvent event) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere(
      (m) =>
          m.timelineType == TimelineEntryType.subagent &&
          m.subagentId == event.agentId,
    );
    if (idx != -1) {
      msgs[idx].subagentContent += event.content;
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Accumulate thinking content from a subagent into its timeline entry.
  void _onSubagentThinkingEvent(
    ChatState chatState,
    SubagentThinkingEvent event,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere(
      (m) =>
          m.timelineType == TimelineEntryType.subagent &&
          m.subagentId == event.agentId,
    );
    if (idx != -1) {
      msgs[idx].subagentThinking += event.content;
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Record a tool call result from a subagent into its timeline entry.
  void _onSubagentToolResultEvent(
    ChatState chatState,
    SubagentToolResultEvent event,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere(
      (m) =>
          m.timelineType == TimelineEntryType.subagent &&
          m.subagentId == event.agentId,
    );
    if (idx != -1) {
      msgs[idx].subagentToolCalls.add(
        ToolCallRecord(
          toolName: event.toolName,
          status: _parseToolStatus(event.status),
          arguments: event.arguments,
          result: event.result,
        ),
      );
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Handle status updates from a subagent (currently just triggers rebuild).
  void _onSubagentStatusEvent(ChatState chatState, SubagentStatusEvent event) {
    // Status events from subagents don't need special handling yet;
    // the UI will reflect them via the subagent timeline entry.
  }

  /// Run ID of the most recent (or current) orchestrator run.
  /// Captured from the `RunStartedEvent` so we can reconnect if the stream drops.
  String? _currentRunId;

  /// Sequence number of the last event received from the current run.
  /// Used as the `since` cursor when requesting event-log replay.
  int _lastSeq = 0;

  /// When `true`, a transient connection error occurred and silent recovery
  /// should be attempted on app resume (via [attemptReconnect]).
  bool _needsReconnect = false;

  /// User message ID from the interrupted stream (for replay status updates).
  String? _disconnectedUserMsgId;

  /// Conversation ID at the time the stream was interrupted.
  String? _disconnectedConversationId;

  /// Whether a transient stream interruption occurred that should be retried
  /// on app resume.
  bool get needsReconnect => _needsReconnect;

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
    _resetReconnectState();
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
      final messages = <ChatMessage>[];
      for (final m in detail.messages) {
        // Convert persisted tool calls into separate timeline entries so
        // they render identically to the streaming path.
        final toolCalls = m.toolCalls?.toList() ?? <dynamic>[];
        for (final tc in toolCalls) {
          messages.add(
            ChatMessage(
              id: 'toolcall-${tc.name}-${m.id}',
              role: 'assistant',
              content: '',
              timelineType: TimelineEntryType.toolCall,
              toolCalls: [
                ToolCallRecord(
                  toolName: tc.name,
                  status: _parseToolStatus(tc.status),
                  arguments: tc.arguments?.asMap.cast<String, dynamic>(),
                  result: tc.result,
                ),
              ],
            ),
          );
        }
        messages.add(
          ChatMessage(
            id: m.id,
            role: m.role,
            content: m.content,
            ttsAvailable: m.ttsAvailable,
            attachments: m.attachments
                ?.map(
                  (a) => ChatAttachment(
                    id: a.id,
                    filename: a.filename,
                    mimeType: a.mimeType,
                    url: a.url,
                  ),
                )
                .toList(),
          ),
        );
      }

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
    _resetReconnectState();
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
  Future<void> sendMessage(
    String message, {
    List<String> attachmentIds = const [],
    List<ChatAttachment> attachments = const [],
  }) async {
    if (message.trim().isEmpty && attachmentIds.isEmpty) return;

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
          PendingMessage(
            text: message,
            conversationId: conversationId,
            attachmentIds: attachmentIds,
            attachments: attachments,
          ),
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
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
        } else if (event is TokenEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          final newContent = chatState.streamingContent + event.token;
          final ms = List<ChatMessage>.from(chatState.messages);
          if (chatState.streamingContent.isEmpty) {
            _markTimelineEntriesStale(ms);
          }
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
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onStatusEvent(chatState, event);
        } else if (event is ToolResultEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onToolResultEvent(chatState, event);
        } else if (event is DoneEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          final ms = List<ChatMessage>.from(chatState.messages);
          final uIdx = ms.indexWhere((m) => m.id == userMsgId);
          if (uIdx != -1) {
            ms[uIdx] = ms[uIdx].copyWith(status: MessageStatus.ok);
          }
          final aIdx = ms.indexWhere((m) => m.id == 'assistant-streaming');
          if (aIdx != -1) {
            ms[aIdx] = ChatMessage(
              id: 'assistant-${DateTime.now().millisecondsSinceEpoch}',
              role: 'assistant',
              content: event.content,
            );
          }
          state = AsyncData(
            ChatState(
              conversationId: conversationId,
              messages: ms,
              pendingQueue: chatState.pendingQueue,
            ),
          );
          return true;
        } else if (event is AgentErrorEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          final ms = List<ChatMessage>.from(chatState.messages)
            ..removeWhere((m) => m.id == 'assistant-streaming');
          final uIdx = ms.indexWhere((m) => m.id == userMsgId);
          if (uIdx != -1) {
            ms[uIdx] = ms[uIdx].copyWith(status: MessageStatus.failed);
          }
          state = AsyncData(chatState.copyWith(messages: ms, isSending: false));
          return false;
        } else if (event is ErrorEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
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

        await _streamMessage(
          pending.text,
          pending.conversationId,
          attachmentIds: pending.attachmentIds,
          attachments: pending.attachments,
        );
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

    // Create a broadcast stream controller for token-by-token rendering.
    final tokenController = StreamController<String>.broadcast();

    // Show a placeholder user message while transcribing.
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    final userMsg = ChatMessage(
      id: userMsgId,
      role: 'user',
      content: '🎤 Voice message',
      status: MessageStatus.sending,
      audioBytes: audioBytes,
      audioMimeType: mimeType,
    );
    final assistantPlaceholder = ChatMessage(
      id: 'assistant-streaming',
      role: 'assistant',
      content: '',
      isStreaming: true,
      tokenStream: tokenController.stream,
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
          tokenController.add(event.token);
          final newContent = chatState.streamingContent + event.token;
          final msgs = List<ChatMessage>.from(chatState.messages);
          if (chatState.streamingContent.isEmpty) {
            _markTimelineEntriesStale(msgs);
          }
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
          _onStatusEvent(chatState, event);
        } else if (event is ToolResultEvent) {
          _onToolResultEvent(chatState, event);
        } else if (event is ThinkingEvent) {
          _onThinkingEvent(chatState, event);
        } else if (event is SubagentStartedEvent) {
          _onSubagentStartedEvent(chatState, event);
        } else if (event is SubagentCompletedEvent) {
          _onSubagentCompletedEvent(chatState, event);
        } else if (event is SubagentTokenEvent) {
          _onSubagentTokenEvent(chatState, event);
        } else if (event is SubagentThinkingEvent) {
          _onSubagentThinkingEvent(chatState, event);
        } else if (event is SubagentToolResultEvent) {
          _onSubagentToolResultEvent(chatState, event);
        } else if (event is SubagentStatusEvent) {
          _onSubagentStatusEvent(chatState, event);
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
          unawaited(tokenController.close());
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
            msgs[idx] = ChatMessage(
              id: assistantId,
              role: 'assistant',
              content: event.content,
              audioId: placeholder.audioId,
              ttsAvailable: placeholder.ttsAvailable,
            );
          }
          state = AsyncData(
            ChatState(
              conversationId: conversationId,
              messages: msgs,
              pendingQueue: chatState.pendingQueue,
            ),
          );
          return;
        } else if (event is ErrorEvent) {
          unawaited(tokenController.close());
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
      unawaited(tokenController.close());

      // Transient error with a run ID: defer recovery to app resume.
      if (_isTransientError(e) && _currentRunId != null) {
        _needsReconnect = true;
        _disconnectedUserMsgId = userMsgId;
        _disconnectedConversationId = conversationId;

        final chatState = state.value ?? const ChatState();
        final msgs = List<ChatMessage>.from(chatState.messages)
          ..removeWhere((m) => m.id == 'assistant-streaming');
        state = AsyncData(
          chatState.copyWith(
            messages: msgs,
            isSending: false,
            streamingContent: '',
            clearStatusMessage: true,
          ),
        );
        return;
      }

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
          error: _friendlyErrorMessage(e),
        ),
      );
    }
  }

  /// Stream a single [message] to the server and update state from SSE events.
  Future<void> _streamMessage(
    String message,
    String conversationId, {
    List<String> attachmentIds = const [],
    List<ChatAttachment> attachments = const [],
  }) async {
    final api = _api;
    if (api == null) return;

    _cancelled = false;
    _currentRunId = null;
    _lastSeq = 0;
    _resetReconnectState();
    final current = state.value ?? const ChatState();

    // Create broadcast stream controllers for token-by-token rendering.
    final tokenController = StreamController<String>.broadcast();
    _thinkingController = StreamController<String>.broadcast();

    // Add user message with status=sending and assistant streaming placeholder.
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    final userMsg = ChatMessage(
      id: userMsgId,
      role: 'user',
      content: message,
      status: MessageStatus.sending,
      attachments: attachments.isNotEmpty ? attachments : null,
    );
    final assistantPlaceholder = ChatMessage(
      id: 'assistant-streaming',
      role: 'assistant',
      content: '',
      isStreaming: true,
      tokenStream: tokenController.stream,
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
      await for (final event in api.streamMessages(
        conversationId,
        message,
        attachmentIds: attachmentIds.isNotEmpty ? attachmentIds : null,
      )) {
        if (_cancelled) break;
        final chatState = state.value ?? const ChatState();

        if (event is RunStartedEvent) {
          // Capture the run ID for potential reconnect; don't change UI.
          _currentRunId = event.runId;
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          continue;
        } else if (event is TokenEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          tokenController.add(event.token);
          final newContent = chatState.streamingContent + event.token;
          final msgs = List<ChatMessage>.from(chatState.messages);
          // On first token, mark all timeline entries as stale (answer is
          // streaming — collapse previous thinking/tool/subagent entries).
          if (chatState.streamingContent.isEmpty) {
            _markTimelineEntriesStale(msgs);
          }
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
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onStatusEvent(chatState, event);
        } else if (event is ToolResultEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onToolResultEvent(chatState, event);
        } else if (event is ThinkingEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onThinkingEvent(chatState, event);
        } else if (event is SubagentStartedEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onSubagentStartedEvent(chatState, event);
        } else if (event is SubagentCompletedEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onSubagentCompletedEvent(chatState, event);
        } else if (event is DoneEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          unawaited(tokenController.close());
          _closeThinkingController();
          final msgs = List<ChatMessage>.from(chatState.messages);
          // Mark thinking entries as complete (no longer streaming).
          for (var i = 0; i < msgs.length; i++) {
            if (msgs[i].timelineType == TimelineEntryType.thinking &&
                msgs[i].isStreaming) {
              msgs[i] = msgs[i].copyWith(isStreaming: false);
            }
          }
          // Mark user message as successfully acknowledged.
          final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
          if (userIdx != -1) {
            msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.ok);
          }
          // Replace streaming placeholder with final assistant message,
          // preserving audio id. Tool calls are separate timeline entries.
          final assistantId =
              event.messageId ??
              'assistant-${DateTime.now().millisecondsSinceEpoch}';
          final idx = msgs.indexWhere((m) => m.id == 'assistant-streaming');
          if (idx != -1) {
            final placeholder = msgs[idx];
            msgs[idx] = ChatMessage(
              id: assistantId,
              role: 'assistant',
              content: event.content,
              audioId: placeholder.audioId,
              ttsAvailable: placeholder.ttsAvailable,
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
          return;
        } else if (event is AudioReadyEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
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
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          unawaited(tokenController.close());
          _closeThinkingController();
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

      // Stream ended without DoneEvent — close token controller and treat
      // accumulated buffer as final.
      unawaited(tokenController.close());
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
      unawaited(tokenController.close());
      // Attempt immediate replay (works for brief network blips).
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

      // Transient error with a run ID: defer recovery to app resume
      // instead of showing an error banner.
      if (_isTransientError(e) && _currentRunId != null) {
        _needsReconnect = true;
        _disconnectedUserMsgId = userMsgId;
        _disconnectedConversationId = conversationId;

        final chatState = state.value ?? const ChatState();
        final msgs = List<ChatMessage>.from(chatState.messages)
          ..removeWhere((m) => m.id == 'assistant-streaming');
        state = AsyncData(
          chatState.copyWith(
            messages: msgs,
            isSending: false,
            streamingContent: '',
            clearStatusMessage: true,
          ),
        );
        return;
      }

      // Non-transient error or no run ID: show error immediately.
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
          error: _friendlyErrorMessage(e),
        ),
      );
    }
  }

  /// Returns `true` if [error] looks like a transient connection failure
  /// (iOS backgrounding, network blip) rather than a permanent server error.
  static bool _isTransientError(Object error) {
    if (error is DioException &&
        error.type == DioExceptionType.connectionError) {
      return true;
    }
    final msg = error.toString();
    return msg.contains('HttpException') ||
        msg.contains('Connection closed') ||
        msg.contains('Connection reset') ||
        msg.contains('SocketException');
  }

  /// Returns a user-friendly error message for stream exceptions.
  static String _friendlyErrorMessage(Object error) {
    if (_isTransientError(error)) {
      return 'Connection lost — tap retry to resend';
    }
    return 'Stream error: $error';
  }

  /// Attempt to silently recover from a transient stream interruption.
  ///
  /// Called from the app lifecycle observer on [AppLifecycleState.resumed].
  /// Tries: 1) replay from last sequence, 2) fetch full conversation history,
  /// 3) show error only if both fail.
  Future<void> attemptReconnect() async {
    if (!_needsReconnect) return;

    // Clear immediately to prevent re-entrant calls.
    _needsReconnect = false;

    final api = _api;
    final conversationId = _disconnectedConversationId;
    final userMsgId = _disconnectedUserMsgId;
    final runId = _currentRunId;

    _disconnectedUserMsgId = null;
    _disconnectedConversationId = null;

    if (api == null || conversationId == null) return;

    // Strategy 1: Replay from last event sequence.
    if (runId != null && userMsgId != null) {
      try {
        final replayed = await _replayRun(
          api,
          conversationId,
          userMsgId,
          runId,
          _lastSeq,
        );
        if (replayed) return;
      } catch (_) {
        // Replay failed — fall through to history fetch.
      }
    }

    // Strategy 2: Fetch full conversation history.
    // Save current messages in case loadConversation fails and wipes them.
    final preLoadMessages = List<ChatMessage>.from(state.value?.messages ?? []);
    await loadConversation(conversationId);
    // loadConversation catches errors internally — check if it succeeded.
    final afterLoad = state.value;
    if (afterLoad != null && afterLoad.error == null) return;

    // Both strategies failed: restore messages and show friendly error.
    final msgs = List<ChatMessage>.from(preLoadMessages);
    if (userMsgId != null) {
      final userIdx = msgs.indexWhere((m) => m.id == userMsgId);
      if (userIdx != -1) {
        msgs[userIdx] = msgs[userIdx].copyWith(status: MessageStatus.failed);
      }
    }
    state = AsyncData(
      ChatState(
        conversationId: conversationId,
        messages: msgs,
        error: 'Connection lost — tap retry to resend',
      ),
    );
  }

  /// Clears deferred reconnection state.
  void _resetReconnectState() {
    _needsReconnect = false;
    _disconnectedUserMsgId = null;
    _disconnectedConversationId = null;
  }
}

/// Provider for [ChatNotifier].
final chatProvider = AsyncNotifierProvider.autoDispose<ChatNotifier, ChatState>(
  ChatNotifier.new,
);
