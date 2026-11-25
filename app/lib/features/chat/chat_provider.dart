import 'dart:async';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';
import 'chat_history_mapping.dart';
import 'chat_models.dart';
import 'chat_stream_errors.dart';
import 'chat_timeline_reducers.dart';
import 'conversation_list_provider.dart';

export 'chat_history_mapping.dart';
export 'chat_models.dart';
export 'conversation_list_provider.dart';

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

  /// Update the in-flight turn-status snapshot to reflect a newly-received
  /// SSE event of [kind]. No-op when no turn is in flight (snapshot null).
  ///
  /// [toolName] populates `currentToolName` when relevant (e.g. on
  /// `tool_result` or `subagent_started`). `TokenEvent` is handled
  /// inline in `_streamMessage` so the tool name clears alongside the
  /// streaming-content mutation.
  void _bumpTurnStatus(TurnEventKind kind, {String? toolName}) {
    final s = state.value;
    if (s == null) return;
    final prev = s.currentTurnStatus;
    if (prev == null) return;
    state = AsyncData(
      s.copyWith(
        currentTurnStatus: prev.copyWith(
          lastEventKind: kind,
          lastEventAt: DateTime.now(),
          currentToolName: toolName ?? prev.currentToolName,
        ),
      ),
    );
  }

  /// Create a tool-call timeline entry and insert it before the streaming
  /// assistant placeholder. Called from both [_streamMessage] and
  /// [_streamVoiceMessage] when a [StatusEvent] is received.
  void _onStatusEvent(ChatState chatState, StatusEvent event) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    applyStatusEvent(msgs, event);
    state = AsyncData(
      chatState.copyWith(messages: msgs, statusMessage: event.message),
    );
  }

  /// Resolve the pending tool-call timeline entry for [event] and update state.
  /// Called from both [_streamMessage] and [_streamVoiceMessage].
  void _onToolResultEvent(ChatState chatState, ToolResultEvent event) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    applyToolResultEvent(msgs, event);
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
    applyThinkingEvent(msgs, event, tokenStream: _thinkingController?.stream);
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Apply [reduce] to a copy of the message list and publish the result.
  void _reduceMessages(
    ChatState chatState,
    void Function(List<ChatMessage> msgs) reduce,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    reduce(msgs);
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

  /// Maximum number of replay retries with exponential backoff before
  /// deferring to app-resume reconnection.
  static const _maxStreamRetries = 3;

  /// Base delay for exponential backoff between retries (1s, 2s, 4s).
  static const _baseRetryDelay = Duration(seconds: 1);

  /// How long to wait after Send for the first non-[RunStartedEvent] from
  /// the SSE stream before treating the stream as stalled and falling back
  /// to a direct conversation fetch.
  ///
  /// Motivated by iOS Dio / dart:io HttpClient buffering: the response
  /// headers arrive (so [RunStartedEvent] fires from the `X-Run-Id`
  /// header) but no body chunks reach the parser until the connection
  /// closes, leaving the user staring at "jumpy dots" until the byte-
  /// level 90 s heartbeat finally trips. 12 s is short enough to feel
  /// responsive and long enough to absorb a brief first-token delay.
  static const _initialStallTimeout = Duration(seconds: 12);

  /// Whether a transient stream interruption occurred that should be retried
  /// on app resume.
  bool get needsReconnect => _needsReconnect;

  @override
  Future<ChatState> build() async {
    return const ChatState();
  }

  ApiClient? get _api => ref.read(apiClientProvider);

  /// Remove a queued message at [index] from [ChatState.pendingQueue].
  ///
  /// No-op when [index] is out of bounds. The actively-streaming message
  /// is NOT in the queue (it was popped before `_streamMessage` started),
  /// so this never aborts the in-flight turn.
  ///
  /// Used by the queued-message bubble's "Remove from queue" action sheet.
  void removeFromQueue(int index) {
    final current = state.value ?? const ChatState();
    if (index < 0 || index >= current.pendingQueue.length) return;
    final next = List<PendingMessage>.from(current.pendingQueue)
      ..removeAt(index);
    state = AsyncData(current.copyWith(pendingQueue: next));
  }

  /// Cancel the current in-progress streaming response.
  ///
  /// Queued messages in [ChatState.pendingQueue] are preserved and will
  /// continue to drain after the cancel.
  void cancelStream() {
    _cancelled = true;
    final current = state.value ?? const ChatState();
    final msgs = List<ChatMessage>.from(current.messages)
      ..removeWhere((m) => m.id == streamingPlaceholderId);
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
      final messages = chatMessagesFromHistory(detail.messages);

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

  /// Ask the server to cancel the currently in-flight turn.
  ///
  /// Best-effort: does nothing if no turn is in flight (no run id captured)
  /// or if the client isn't connected. The server-side cancellation
  /// propagates asynchronously — the SSE stream emits an `agent_error` event
  /// with `{"reason": "cancelled"}` and the await-for loop here cleans up
  /// state the same way it would for any other agent-terminated turn.
  ///
  /// Returns the post-cancel server-reported state for callers that want
  /// to wait on the reconciled result; returns `null` when the cancel
  /// call could not be made (no run id, no client, or transport error).
  Future<TurnState?> requestCancelTurn() async {
    final api = _api;
    final runId = _currentRunId;
    final convId = state.value?.conversationId;
    if (api == null || runId == null || convId == null) {
      return null;
    }
    return api.cancelTurn(convId, runId);
  }

  /// Set the active conversation ID without loading history.
  void setConversationId(String id) {
    state = AsyncData(
      (state.value ?? const ChatState()).copyWith(conversationId: id),
    );
  }

  /// Creates a conversation, selects it, and returns its id. Surfaces the
  /// failure on state and returns `null` when creation fails.
  Future<String?> _createConversation(ApiClient api) async {
    final current = state.value ?? const ChatState();
    try {
      final response = await api.conversations.createConversation(
        createConversationRequest: CreateConversationRequest((b) => b),
      );
      final conv = response.data!;
      ref.read(conversationListProvider.notifier).prependConversation(conv);
      state = AsyncData(
        (state.value ?? const ChatState()).copyWith(conversationId: conv.id),
      );
      return conv.id;
    } catch (e) {
      state = AsyncData(
        current.copyWith(error: 'Failed to create conversation: $e'),
      );
      return null;
    }
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

    final conversationId =
        state.value?.conversationId ?? await _createConversation(api);
    if (conversationId == null) return;

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

    final conversationId =
        state.value?.conversationId ?? await _createConversation(api);
    if (conversationId == null) return;

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
    setMessageStatus(msgs, userMsgId, MessageStatus.sending);
    final placeholder = ChatMessage(
      id: streamingPlaceholderId,
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
            markTimelineEntriesStale(ms);
          }
          final idx = ms.indexWhere((m) => m.id == streamingPlaceholderId);
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
          setMessageStatus(ms, userMsgId, MessageStatus.ok);
          final aIdx = ms.indexWhere((m) => m.id == streamingPlaceholderId);
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
            ..removeWhere((m) => m.id == streamingPlaceholderId);
          setMessageStatus(ms, userMsgId, MessageStatus.failed);
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
          ..removeWhere((m) => m.id == streamingPlaceholderId);
        setMessageStatus(ms, userMsgId, MessageStatus.failed);
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
      id: streamingPlaceholderId,
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

    // Arm the same initial-stall watchdog used by the text path. The voice
    // SSE response carries TranscriptEvent + token/status events; any event
    // counts as progress. Until the first event arrives, a quiet interval
    // of [_initialStallTimeout] probes the server: a `running` reply keeps
    // the sink open (iOS Dio buffering safety net), anything else recovers.
    var sawProgress = false;
    final source = api
        .sendVoiceMessage(conversationId, audioBytes, mimeType)
        .timeout(
          _initialStallTimeout,
          onTimeout: (sink) {
            if (sawProgress) return;
            unawaited(
              _probeOrRecover(
                conversationId: conversationId,
                userMsgId: userMsgId,
                sink: sink,
                sawProgress: () => sawProgress,
              ),
            );
          },
        );

    try {
      await for (final event in source) {
        if (_cancelled) break;
        sawProgress = true;
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
            markTimelineEntriesStale(msgs);
          }
          final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
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
          _reduceMessages(
            chatState,
            (msgs) => applySubagentStartedEvent(msgs, event),
          );
        } else if (event is SubagentCompletedEvent) {
          _reduceMessages(
            chatState,
            (msgs) => applySubagentCompletedEvent(msgs, event),
          );
        } else if (event is SubagentTokenEvent) {
          _reduceMessages(
            chatState,
            (msgs) => applySubagentTokenEvent(msgs, event),
          );
        } else if (event is SubagentThinkingEvent) {
          _reduceMessages(
            chatState,
            (msgs) => applySubagentThinkingEvent(msgs, event),
          );
        } else if (event is SubagentToolResultEvent) {
          _reduceMessages(
            chatState,
            (msgs) => applySubagentToolResultEvent(msgs, event),
          );
        } else if (event is SubagentStatusEvent) {
          _onSubagentStatusEvent(chatState, event);
        } else if (event is AudioReadyEvent) {
          // Store audioId on the streaming assistant message for auto-play.
          final msgs = List<ChatMessage>.from(chatState.messages);
          final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
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
          setMessageStatus(msgs, userMsgId, MessageStatus.ok);
          // Preserve tool call records and audio id on the final message.
          final assistantId =
              event.messageId ??
              'assistant-${DateTime.now().millisecondsSinceEpoch}';
          final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
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
            ..removeWhere((m) => m.id == streamingPlaceholderId);
          setMessageStatus(msgs, userMsgId, MessageStatus.failed);
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              isSending: false,
              streamingContent: '',
              error: event.message,
              clearCurrentTurnStatus: true,
            ),
          );
          return;
        }
      }
    } catch (e) {
      unawaited(tokenController.close());

      // Transient error with a run ID: retry with exponential backoff.
      if (isTransientStreamError(e) && _currentRunId != null) {
        for (var attempt = 0; attempt < _maxStreamRetries; attempt++) {
          await Future<void>.delayed(_baseRetryDelay * (1 << attempt));
          if (_cancelled || !ref.mounted) return;

          final replayed = await _replayRun(
            api,
            conversationId,
            userMsgId,
            _currentRunId!,
            _lastSeq,
          );
          if (replayed) return;
        }

        // All retries exhausted — defer to app resume.
        _needsReconnect = true;
        _disconnectedUserMsgId = userMsgId;
        _disconnectedConversationId = conversationId;

        final chatState = state.value ?? const ChatState();
        final msgs = List<ChatMessage>.from(chatState.messages)
          ..removeWhere((m) => m.id == streamingPlaceholderId);
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
        ..removeWhere((m) => m.id == streamingPlaceholderId);
      setMessageStatus(msgs, userMsgId, MessageStatus.failed);
      state = AsyncData(
        chatState.copyWith(
          messages: msgs,
          isSending: false,
          streamingContent: '',
          error: friendlyStreamErrorMessage(e),
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
      id: streamingPlaceholderId,
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

    // Arm the initial-stall watchdog as a per-event timeout on the SSE
    // stream. Until we see a UI-visible event (anything other than
    // [RunStartedEvent]), a quiet interval of [_initialStallTimeout]
    // probes the server to disambiguate iOS Dio buffering (server still
    // running — keep the sink open) from a dead turn (recover via
    // [_recoverStalledStream]). After progress has been observed, the
    // byte-level heartbeat (90 s) handles longer silences that may
    // legitimately occur during tool execution.
    var sawProgress = false;
    final source = api
        .streamMessages(
          conversationId,
          message,
          attachmentIds: attachmentIds.isNotEmpty ? attachmentIds : null,
        )
        .timeout(
          _initialStallTimeout,
          onTimeout: (sink) {
            if (sawProgress) return;
            unawaited(
              _probeOrRecover(
                conversationId: conversationId,
                userMsgId: userMsgId,
                sink: sink,
                sawProgress: () => sawProgress,
              ),
            );
          },
        );

    // Stream SSE events.
    try {
      await for (final event in source) {
        if (_cancelled) break;
        final chatState = state.value ?? const ChatState();

        if (event is RunStartedEvent) {
          // Capture the run ID for potential reconnect; don't change UI.
          _currentRunId = event.runId;
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          // Open the turn-status snapshot used by the progress card.
          state = AsyncData(
            chatState.copyWith(
              currentTurnStatus: TurnStatusSnapshot(
                runId: event.runId,
                lastEventKind: TurnEventKind.runStarted,
                lastEventAt: DateTime.now(),
              ),
            ),
          );
          continue;
        }
        // Any non-RunStartedEvent counts as progress — the initial-stall
        // watchdog won't fire on subsequent quiet periods.
        sawProgress = true;
        if (event is TokenEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          tokenController.add(event.token);
          final newContent = chatState.streamingContent + event.token;
          final msgs = List<ChatMessage>.from(chatState.messages);
          // On first token, mark all timeline entries as stale (answer is
          // streaming — collapse previous thinking/tool/subagent entries).
          if (chatState.streamingContent.isEmpty) {
            markTimelineEntriesStale(msgs);
          }
          final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
          if (idx != -1) {
            msgs[idx].content = newContent;
          }
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              streamingContent: newContent,
              clearStatusMessage: true,
              // Resuming text generation — clear any active tool name.
              currentTurnStatus: chatState.currentTurnStatus?.copyWith(
                lastEventKind: TurnEventKind.token,
                lastEventAt: DateTime.now(),
                clearToolName: true,
              ),
            ),
          );
        } else if (event is StatusEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onStatusEvent(chatState, event);
          _bumpTurnStatus(TurnEventKind.status);
        } else if (event is ToolResultEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onToolResultEvent(chatState, event);
          _bumpTurnStatus(TurnEventKind.toolResult, toolName: event.toolName);
        } else if (event is ThinkingEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _onThinkingEvent(chatState, event);
          _bumpTurnStatus(TurnEventKind.thinking);
        } else if (event is SubagentStartedEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _reduceMessages(
            chatState,
            (msgs) => applySubagentStartedEvent(msgs, event),
          );
          _bumpTurnStatus(TurnEventKind.subagentStarted, toolName: event.task);
        } else if (event is SubagentCompletedEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          _reduceMessages(
            chatState,
            (msgs) => applySubagentCompletedEvent(msgs, event),
          );
          _bumpTurnStatus(TurnEventKind.subagentCompleted);
        } else if (event is DoneEvent) {
          _lastSeq = (event.sequenceId ?? _lastSeq) + 1;
          unawaited(tokenController.close());
          _closeThinkingController();
          final msgs = List<ChatMessage>.from(chatState.messages);
          // Mark thinking entries as complete (no longer streaming).
          completeStreamingThinkingEntries(msgs);
          // Mark user message as successfully acknowledged.
          setMessageStatus(msgs, userMsgId, MessageStatus.ok);
          // Replace streaming placeholder with final assistant message,
          // preserving audio id. Tool calls are separate timeline entries.
          final assistantId =
              event.messageId ??
              'assistant-${DateTime.now().millisecondsSinceEpoch}';
          final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
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
          final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
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
            ..removeWhere((m) => m.id == streamingPlaceholderId);
          // Mark user message as failed (keep it in the list for retry).
          setMessageStatus(msgs, userMsgId, MessageStatus.failed);
          state = AsyncData(
            chatState.copyWith(
              messages: msgs,
              isSending: false,
              streamingContent: '',
              error: event.message,
              clearCurrentTurnStatus: true,
            ),
          );
          return;
        }
      }

      // Stream ended without DoneEvent — close token controller and treat
      // accumulated buffer as final.
      unawaited(tokenController.close());
      if (!ref.mounted) return;
      final finalState = state.value ?? const ChatState();
      if (finalState.isSending) {
        final msgs = List<ChatMessage>.from(finalState.messages);
        final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
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
        setMessageStatus(msgs, userMsgId, MessageStatus.ok);
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
      _closeThinkingController();
      if (!ref.mounted) return;

      // Transient error with a run ID: retry with exponential backoff,
      // then defer to app-resume reconnection if all retries fail.
      if (isTransientStreamError(e) && _currentRunId != null) {
        for (var attempt = 0; attempt < _maxStreamRetries; attempt++) {
          await Future<void>.delayed(_baseRetryDelay * (1 << attempt));
          if (_cancelled || !ref.mounted) return;

          final replayed = await _replayRun(
            api,
            conversationId,
            userMsgId,
            _currentRunId!,
            _lastSeq,
          );
          if (replayed) return;
        }

        // All retries exhausted — defer to app resume.
        _needsReconnect = true;
        _disconnectedUserMsgId = userMsgId;
        _disconnectedConversationId = conversationId;

        final chatState = state.value ?? const ChatState();
        final msgs = List<ChatMessage>.from(chatState.messages)
          ..removeWhere((m) => m.id == streamingPlaceholderId);
        state = AsyncData(
          chatState.copyWith(
            messages: msgs,
            isSending: false,
            streamingContent: '',
            clearStatusMessage: true,
            clearCurrentTurnStatus: true,
          ),
        );
        return;
      }

      // Non-transient error or no run ID: show error immediately.
      final chatState = state.value ?? const ChatState();
      final msgs = List<ChatMessage>.from(chatState.messages)
        ..removeWhere((m) => m.id == streamingPlaceholderId);
      setMessageStatus(msgs, userMsgId, MessageStatus.failed);
      state = AsyncData(
        chatState.copyWith(
          messages: msgs,
          isSending: false,
          streamingContent: '',
          error: friendlyStreamErrorMessage(e),
          clearCurrentTurnStatus: true,
        ),
      );
    }
  }

  /// Called from the initial-stall watchdog's `onTimeout`. Probes the
  /// server-side turn state to decide whether to keep the SSE sink open
  /// (server still working — most often iOS Dio buffering the chunked
  /// body) or to fall back to [_recoverStalledStream] for terminal /
  /// unknown turns.
  ///
  /// `sawProgress` is a closure-captured getter — the same flag the
  /// caller sets to `true` on the first UI-visible event. We re-check
  /// it after the probe completes because the stream may have woken up
  /// while we were awaiting the server.
  Future<void> _probeOrRecover({
    required String conversationId,
    required String userMsgId,
    required EventSink<StreamEvent> sink,
    required bool Function() sawProgress,
  }) async {
    if (sawProgress() || _cancelled) return;
    final api = _api;
    final runId = _currentRunId;
    if (api == null || runId == null) {
      // RunStarted hasn't fired yet (or no client) — no run id to probe.
      // Fall back to the legacy recovery path so the dots state still
      // resolves within the watchdog window.
      await _recoverStalledStream(conversationId, userMsgId);
      if (!_cancelled) sink.close();
      return;
    }
    final state = await api.turnStatus(conversationId, runId);
    if (sawProgress() || _cancelled) return;
    if (state == TurnState.running) {
      // Server says the turn is still in flight — leave the sink open.
      // The chat-stream-progress-ux card already shows a stalled UI past
      // `kTurnStallThreshold`; the 90 s byte-watchdog is the final
      // safety net for genuinely dead connections.
      return;
    }
    // completed / errored / unknown / null (probe failed) → recover.
    await _recoverStalledStream(conversationId, userMsgId);
    if (!_cancelled) sink.close();
  }

  /// Recover from a stalled SSE stream by fetching the conversation
  /// directly. Triggered by the [_initialStallTimeout] watchdog when the
  /// stream produces no UI-visible event in time — primarily the iOS
  /// case where Dio buffers the chunked body until the connection
  /// closes.
  ///
  /// On success, replaces the streaming placeholder with the server-
  /// persisted message list. On failure, clears the placeholder and
  /// surfaces an error so the user can retry.
  Future<void> _recoverStalledStream(
    String conversationId,
    String userMsgId,
  ) async {
    if (_cancelled || !ref.mounted) return;
    _cancelled = true; // stop the await-for loop on its next iteration

    // Step 1: clear the placeholder synchronously so the user immediately
    // sees that the stream stalled and gets out of the dots state.
    final current = state.value ?? const ChatState();
    _clearStalledPlaceholder(
      current,
      userMsgId,
      error: 'Response is taking longer than expected. Refreshing…',
    );

    // Step 2: in the background, fetch the conversation from the server.
    // The user reported that a manual reload surfaces the actual reply, so
    // a direct GET typically resolves the missing message.
    final api = _api;
    if (api == null) return;
    try {
      final response = await api.conversations.getConversation(
        id: conversationId,
      );
      if (!ref.mounted) return;
      final messages = chatMessagesFromHistory(response.data!.messages);
      final latest = state.value ?? const ChatState();
      // While we were awaiting the conversation fetch, the queue-drain
      // loop may have advanced to the next pending message and started
      // a new stream. Overwriting state here would silently wipe that
      // in-flight placeholder. The new stream is the freshest truth —
      // discard the stale recovery snapshot instead.
      //
      // Reproducer / regression test:
      //   test/unit/chat/chat_provider_test.dart group 12 — "stalled
      //   recovery does not clobber a concurrently-started stream".
      if (latest.isSending) return;
      state = AsyncData(
        ChatState(
          conversationId: conversationId,
          messages: messages,
          pendingQueue: latest.pendingQueue,
        ),
      );
    } catch (_) {
      // Fetch failed — keep the cleared placeholder and the error message
      // already surfaced above so the user can retry.
    }
  }

  void _clearStalledPlaceholder(
    ChatState from,
    String userMsgId, {
    required String error,
  }) {
    if (!ref.mounted) return;
    final msgs = List<ChatMessage>.from(from.messages)
      ..removeWhere((m) => m.id == streamingPlaceholderId);
    setMessageStatus(msgs, userMsgId, MessageStatus.failed);
    state = AsyncData(
      from.copyWith(
        messages: msgs,
        isSending: false,
        streamingContent: '',
        error: error,
        clearCurrentTurnStatus: true,
      ),
    );
  }

  /// Attempt to silently recover from a transient stream interruption.
  ///
  /// Called from the app lifecycle observer on [AppLifecycleState.resumed].
  /// Tries: 1) replay from last sequence, 2) fetch full conversation history,
  /// 3) show error only if both fail.
  ///
  /// Also kicks the queue-drain loop as a safety net: if for any reason
  /// the drain stopped while messages remained queued (e.g. an exception
  /// escaping a finally block, an app-lifecycle transition interacting
  /// with `ref.mounted`), foregrounding the app would otherwise leave
  /// the queue stuck. The byte-level watchdog plus the explicit `cancel`
  /// surface (future work) cover the main paths, but a non-empty
  /// `pendingQueue` arriving at this hook is an unambiguous signal to
  /// re-kick the drain.
  Future<void> attemptReconnect() async {
    if (!_needsReconnect) {
      // No interrupted stream to recover, but still kick the drain if
      // there are queued messages and nothing is processing them. This
      // is a defence-in-depth net for races elsewhere in the state
      // machine.
      if (!_draining && (state.value?.pendingQueue.isNotEmpty ?? false)) {
        unawaited(_drainQueue());
      }
      return;
    }

    // Clear immediately to prevent re-entrant calls.
    _needsReconnect = false;

    final api = _api;
    final conversationId = _disconnectedConversationId;
    final userMsgId = _disconnectedUserMsgId;
    final runId = _currentRunId;

    _disconnectedUserMsgId = null;
    _disconnectedConversationId = null;

    if (api == null || conversationId == null) return;

    // Surface the recovery work as a banner — set BEFORE any await so
    // the UI shows "Reconnecting…" promptly. Cleared in every exit path.
    final current = state.value ?? const ChatState();
    state = AsyncData(current.copyWith(isReconnecting: true));

    try {
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
      // Save current messages in case loadConversation fails and wipes
      // them.
      final preLoadMessages = List<ChatMessage>.from(
        state.value?.messages ?? [],
      );
      await loadConversation(conversationId);
      // loadConversation catches errors internally — check if it
      // succeeded.
      final afterLoad = state.value;
      if (afterLoad != null && afterLoad.error == null) return;

      // Both strategies failed: restore messages and show friendly error.
      final msgs = List<ChatMessage>.from(preLoadMessages);
      if (userMsgId != null) {
        setMessageStatus(msgs, userMsgId, MessageStatus.failed);
      }
      state = AsyncData(
        ChatState(
          conversationId: conversationId,
          messages: msgs,
          error: 'Connection lost — tap retry to resend',
        ),
      );
    } finally {
      // Clear the banner regardless of which branch terminated us.
      final s = state.value;
      if (s != null && s.isReconnecting) {
        state = AsyncData(s.copyWith(isReconnecting: false));
      }
    }
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
