import 'dart:async';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_ui/assistant_ui.dart' show kTurnStallThreshold;
import 'package:assistant_ui/models.dart';
import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/api_client.dart';
import '../../api/models/stream_event.dart';
import '../connection/connection_provider.dart';
import 'chat_announcer.dart';
import 'chat_branch_switch.dart';
import 'chat_history_mapping.dart';
import 'chat_models.dart';
import 'chat_stream_errors.dart';
import 'chat_stream_session.dart';
import 'chat_timeline_reducers.dart';
import 'chat_turn_transitions.dart';
import 'conversation_list_provider.dart';
import 'follow_up_listener.dart';
import 'interaction_actions.dart';
import 'interaction_kinds.dart';
import 'message_feedback_provider.dart';

export 'package:assistant_ui/models.dart';

export 'chat_history_mapping.dart';
export 'chat_models.dart';
export 'conversation_list_provider.dart';

/// Manages the active chat conversation.
class ChatNotifier extends AsyncNotifier<ChatState>
    with ChatBranchSwitching, ChatInteractions {
  final _session = ChatStreamSession();

  /// Waits for follow-up suggestions after the last `done`. Independent of
  /// [_session]: the next turn resets that, this one is only cancelled.
  FollowUpListener? _followUps;

  final _lifecycle = StreamController<ChatLifecycleEvent>.broadcast(sync: true);
  Timer? _stallTimer;

  /// Whether a turn the user is waiting on is in flight. Gates every terminal
  /// lifecycle event, so loading history never announces anything.
  bool _turnInFlight = false;

  /// Turn lifecycle events for screen-reader announcements. Token frames are
  /// not events, and loading a conversation's history emits none.
  Stream<ChatLifecycleEvent> get lifecycle => _lifecycle.stream;

  void _emit(ChatLifecycleEvent Function(DateTime at) build) {
    if (!_lifecycle.isClosed) _lifecycle.add(build(DateTime.now()));
  }

  void _beginLifecycle() {
    _stallTimer?.cancel();
    _stallTimer = null;
    _turnInFlight = true;
    _emit((at) => TurnStarted(at: at));
  }

  /// Ends the in-flight turn, emitting [terminal] when one was tracked.
  void _endLifecycle([ChatLifecycleEvent Function(DateTime at)? terminal]) {
    _stallTimer?.cancel();
    _stallTimer = null;
    final wasInFlight = _turnInFlight;
    _turnInFlight = false;
    if (wasInFlight && terminal != null) _emit(terminal);
  }

  /// Restarts the stall countdown; called on every status update.
  void _armStallTimer() {
    if (!_turnInFlight) return;
    _stallTimer?.cancel();
    _stallTimer = Timer(kTurnStallThreshold, () {
      _stallTimer = null;
      if (_turnInFlight && ref.mounted) _emit((at) => TurnStalled(at: at));
    });
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
    _armStallTimer();
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
    _session.thinking?.add(event.content);

    final msgs = List<ChatMessage>.from(chatState.messages);
    applyThinkingEvent(msgs, event, tokenStream: _session.thinking?.stream);
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Show an interaction the turn now waits on and announce it.
  void _onInteractionRequested(
    ChatState chatState,
    InteractionRequestedEvent event,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    applyInteractionRequested(msgs, event);
    final record = findInteraction(msgs, event.interactionId);
    var next = withInteractionMessages(chatState, msgs);
    if (record != null) {
      next = next.copyWith(lastInteractionRequest: record);
    }
    state = AsyncData(next);
    if (_turnInFlight && record != null) {
      final announcement = interactionKindSpec(record).announcement;
      _emit((at) => TurnAwaitingInput(announcement: announcement, at: at));
    }
  }

  /// End an interaction and let the progress card return to its activity.
  void _onInteractionResolved(
    ChatState chatState,
    InteractionResolvedEvent event,
  ) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    applyInteractionResolved(msgs, event);
    state = AsyncData(withInteractionMessages(chatState, msgs));
    if (_turnInFlight) _emit((at) => TurnInputResolved(at: at));
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

  /// Apply [event] when it is a mid-turn event shared by the text, voice and
  /// replay paths, and report whether it was handled.
  ///
  /// Run start, transcript and terminal events differ per path and stay with
  /// the callers. Token text is also pushed to [tokenSink] for incremental
  /// rendering when the caller has one.
  bool _applyTurnEvent(
    ChatState chatState,
    StreamEvent event, {
    StreamController<String>? tokenSink,
  }) {
    switch (event) {
      case TokenEvent():
        tokenSink?.add(event.token);
        _onTokenEvent(chatState, event);
      case StatusEvent():
        _onStatusEvent(chatState, event);
        _bumpTurnStatus(TurnEventKind.status);
      case ToolResultEvent():
        _onToolResultEvent(chatState, event);
        _bumpTurnStatus(TurnEventKind.toolResult, toolName: event.toolName);
      case ThinkingEvent():
        _onThinkingEvent(chatState, event);
        _bumpTurnStatus(TurnEventKind.thinking);
      case SubagentStartedEvent():
        _reduceMessages(
          chatState,
          (msgs) => applySubagentStartedEvent(msgs, event),
        );
        _bumpTurnStatus(TurnEventKind.subagentStarted, toolName: event.task);
      case SubagentCompletedEvent():
        _reduceMessages(
          chatState,
          (msgs) => applySubagentCompletedEvent(msgs, event),
        );
        _bumpTurnStatus(TurnEventKind.subagentCompleted);
      case SubagentTokenEvent():
        _reduceMessages(
          chatState,
          (msgs) => applySubagentTokenEvent(msgs, event),
        );
      case SubagentThinkingEvent():
        _reduceMessages(
          chatState,
          (msgs) => applySubagentThinkingEvent(msgs, event),
        );
      case SubagentToolResultEvent():
        _reduceMessages(
          chatState,
          (msgs) => applySubagentToolResultEvent(msgs, event),
        );
      case SubagentStatusEvent():
        // Subagent status lines are not rendered; the subagent timeline entry
        // already reflects progress through its token and tool-result events.
        break;
      case AudioReadyEvent():
        _onAudioReadyEvent(chatState, event);
      case InteractionRequestedEvent():
        _onInteractionRequested(chatState, event);
      case InteractionResolvedEvent():
        _onInteractionResolved(chatState, event);
      default:
        return false;
    }
    return true;
  }

  /// Append a streamed answer token to the assistant placeholder.
  void _onTokenEvent(ChatState chatState, TokenEvent event) {
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
    _armStallTimer();
  }

  /// Replace the streaming placeholder with the final assistant message and
  /// reset per-turn state, keeping the pending queue.
  void _finishTurn(
    ChatState chatState,
    DoneEvent event, {
    required String conversationId,
    required String userMsgId,
    required TurnSource source,
  }) {
    _session.closeThinking();
    state = AsyncData(
      completedTurnState(
        chatState,
        conversationId: conversationId,
        userMsgId: userMsgId,
        assistantId:
            event.messageId ??
            'assistant-${DateTime.now().millisecondsSinceEpoch}',
        content: event.content,
        timing: event.timing,
      ),
    );
    _endLifecycle(
      (at) => TurnCompleted(content: event.content, source: source, at: at),
    );
  }

  /// Drop the streaming placeholder, mark the user message failed (kept for
  /// retry) and surface the server-reported error.
  void _failTurn(
    ChatState chatState,
    ErrorEvent event, {
    required String userMsgId,
  }) {
    _session.closeThinking();
    state = AsyncData(
      abortedTurnState(
        chatState,
        failedUserMsgId: userMsgId,
        error: _stoppedBeforeRunId ? null : event.message,
        clearTurnStatus: true,
      ),
    );
    _endLifecycle(
      _stoppedBeforeRunId
          ? (at) => TurnStopped(at: at)
          : (at) => TurnFailed(message: event.message, at: at),
    );
  }

  /// Whether the user tapped Stop before the turn got a run id: the failure
  /// is then the expected outcome, so it is not surfaced as an error banner.
  bool get _stoppedBeforeRunId =>
      _session.stopRequested && _session.runId == null;

  /// Reconcile a terminal `agent_error` frame on any of the three stream
  /// paths: a cancelled turn keeps its partial reply as a stopped message,
  /// anything else fails the turn like an [ErrorEvent].
  void _onAgentError(
    ChatState chatState,
    AgentErrorEvent event, {
    required String userMsgId,
  }) {
    _session.closeThinking();
    _session.terminalFrameSeen = true;
    state = AsyncData(switch (event.reason) {
      AgentErrorReason.cancelled => stoppedTurnState(
        chatState,
        userMsgId: userMsgId,
        partialContent: event.partialContent,
        messageId: event.messageId,
        userMessagePersisted: event.userMessagePersisted,
      ),
      AgentErrorReason.error => abortedTurnState(
        chatState,
        failedUserMsgId: userMsgId,
        error: event.message,
        clearTurnStatus: true,
      ),
    });
    _endLifecycle(switch (event.reason) {
      AgentErrorReason.cancelled => (at) => TurnStopped(at: at),
      AgentErrorReason.error => (at) => TurnFailed(
        message: event.message,
        at: at,
      ),
    });
  }

  /// Remember the run id for replay and Stop, and deliver a Stop that was
  /// tapped before the id was known.
  void _captureRunId(RunStartedEvent event) {
    _session.runId = event.runId;
    _session.advance(event);
    if (_session.stopRequested) unawaited(_requestServerCancel());
  }

  /// Open the turn-status snapshot the progress card reads.
  ///
  /// Both the text and the voice path open it on `RunStartedEvent`; an
  /// interaction requested later needs its `runId` to submit an answer.
  void _openTurnStatus(RunStartedEvent event) {
    final chatState = state.value ?? const ChatState();
    state = AsyncData(
      chatState.copyWith(
        currentTurnStatus: TurnStatusSnapshot(
          runId: event.runId,
          lastEventKind: TurnEventKind.runStarted,
          lastEventAt: DateTime.now(),
        ),
      ),
    );
    _armStallTimer();
  }

  /// Store the synthesized-audio id on the streaming assistant placeholder.
  void _onAudioReadyEvent(ChatState chatState, AudioReadyEvent event) {
    final msgs = List<ChatMessage>.from(chatState.messages);
    final idx = msgs.indexWhere((m) => m.id == streamingPlaceholderId);
    if (idx != -1) {
      msgs[idx] = msgs[idx].copyWith(
        audioId: event.audioId,
        ttsAvailable: true,
      );
    }
    state = AsyncData(chatState.copyWith(messages: msgs));
  }

  /// Start waiting for follow-up suggestions on [runId]'s replay stream after
  /// its `done` frame. Does nothing without a message id to tie them to.
  void _startFollowUps(
    ApiClient api,
    DoneEvent done, {
    required String conversationId,
    required String? runId,
  }) {
    final messageId = done.messageId;
    if (messageId == null || runId == null) return;
    _cancelFollowUps();
    _followUps = FollowUpListener(
      api: api,
      conversationId: conversationId,
      runId: runId,
      since: _session.lastSeq,
      messageId: messageId,
      wrapSource: _session.untilDetached,
      onSuggestions: (prompts) =>
          _applySuggestions(conversationId, messageId, prompts),
    )..start();
  }

  void _cancelFollowUps() {
    _followUps?.cancel();
    _followUps = null;
  }

  /// Show [prompts] only while [messageId] is still the last assistant
  /// message of the idle conversation they were generated for.
  void _applySuggestions(
    String conversationId,
    String messageId,
    List<String> prompts,
  ) {
    final current = state.value;
    if (!ref.mounted || current == null) return;
    if (current.isSending || current.conversationId != conversationId) return;
    final lastAssistant = current.messages
        .where((m) => m.role == 'assistant')
        .lastOrNull;
    if (lastAssistant?.id != messageId) return;
    state = AsyncData(current.copyWith(suggestions: prompts));
  }

  /// Whether a transient stream interruption occurred that should be retried
  /// on app resume.
  bool get needsReconnect => _session.needsReconnect;

  @override
  Future<ChatState> build() async {
    ref.onDispose(() {
      _cancelFollowUps();
      _session.cancelStopTimer();
      _stallTimer?.cancel();
      unawaited(_lifecycle.close());
    });
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

  /// Stop the in-flight turn by asking the server to cancel it.
  ///
  /// The stream stays open: the terminal frame (cancelled, or `done` when the
  /// turn won the race) reconciles the turn. [ChatState.isStopping] is true
  /// until then, and further calls are ignored. Before the run id is known
  /// the cancel is deferred to `RunStartedEvent`. Queued messages are kept.
  void stopTurn() {
    final current = state.value;
    if (current == null || !current.isSending || current.isStopping) return;
    _session.stopRequested = true;
    state = AsyncData(current.copyWith(isStopping: true));
    // The ceiling counts from the tap, whether or not the run id is known.
    _session.stopTimer = Timer(ChatStreamSession.stopGrace, _onStopGrace);
    if (_session.runId != null) unawaited(_requestServerCancel());
  }

  /// [ChatStreamSession.stopGrace] expired without a terminal frame. Probe the
  /// run once: a running (or unreachable) server is wedged, so the partial is
  /// kept as the stopped reply; anything else has an outcome the stream failed
  /// to deliver, so show what the server persisted.
  Future<void> _onStopGrace() async {
    if (!ref.mounted || _session.detached) return;
    final turn = _session.turnSeq;
    final api = _api;
    final runId = _session.runId;
    final conversationId = _session.conversationId;
    if (api == null || runId == null || conversationId == null) {
      await _detachTurn(error: _noRunIdStopError);
      return;
    }
    final status = await api.turnStatus(conversationId, runId);
    if (!_stopStillPending(turn)) return;
    if (_isLive(status) || status == null) {
      await _detachTurn(error: _unconfirmedStopError);
    } else {
      await _detachTurn(refresh: true);
    }
  }

  /// Whether [state] says the turn is still alive. A turn waiting for the
  /// owner's answer is as alive as a running one, and is silent for as long.
  static bool _isLive(TurnState? state) =>
      state == TurnState.running || state == TurnState.awaitingInput;

  /// Retries of an unanswered cancel request, after the first attempt.
  static const _cancelRetries = 2;

  static const _noRunIdStopError =
      "Couldn't confirm the stop. The reply may still finish.";
  static const _unconfirmedStopError =
      "The server hasn't confirmed the stop. The reply may still finish.";
  static const _unreachableStopError =
      "Couldn't reach the server to stop this reply. It may still finish.";

  /// Whether the Stop of turn [turn] is still waiting for its outcome.
  bool _stopStillPending(int turn) =>
      ref.mounted &&
      _session.turnSeq == turn &&
      !_session.detached &&
      (state.value?.isStopping ?? false);

  /// Give up on the server's terminal frame: end the stream loops at once,
  /// keep the streamed text as a stopped reply (the user message stays `ok`,
  /// the server may have it), surface [error] and, with [refresh], replace
  /// the messages with the server's. The queue drains right after.
  Future<void> _detachTurn({String? error, bool refresh = false}) async {
    final current = state.value;
    if (!ref.mounted || _session.detached || current == null) return;
    final conversationId = _session.conversationId;
    _session.detach();
    _session.closeThinking();
    final stopped = stoppedTurnState(
      current,
      userMsgId: _session.userMsgId ?? '',
      partialContent: '',
    );
    state = AsyncData(error == null ? stopped : stopped.copyWith(error: error));
    _endLifecycle((at) => TurnStopped(at: at));
    if (refresh && conversationId != null) {
      await _refreshFromServer(conversationId);
    }
  }

  /// Load an existing conversation by ID.
  Future<void> loadConversation(String conversationId) async {
    conversationChanged();
    _session.clearDisconnected();
    _cancelFollowUps();
    state = AsyncData(
      (state.value ?? const ChatState()).copyWith(
        isLoadingHistory: true,
        clearError: true,
        clearSuggestions: true,
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
      final messages = _withPendingFeedback(
        chatMessagesFromHistory(detail.messages),
      );

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

  // -- Message feedback ------------------------------------------------------

  /// Let a message with queued or in-flight feedback calls keep its
  /// optimistic value over the value a history load just seeded.
  List<ChatMessage> _withPendingFeedback(List<ChatMessage> messages) {
    final feedback = ref.read(messageFeedbackControllerProvider.notifier);
    for (final m in messages) {
      if (feedback.isPending(m.id)) m.feedback = feedback.pendingValue(m.id);
    }
    return messages;
  }

  ChatMessage? _messageById(String messageId) =>
      state.value?.messages.where((m) => m.id == messageId).firstOrNull;

  /// Show [feedback] on the message at once and send it to the server;
  /// `null` clears the vote. Returns `false` when the message is not in the
  /// current conversation.
  bool setMessageFeedback(String messageId, MessageFeedback? feedback) {
    final current = state.value;
    final conversationId = current?.conversationId;
    final message = _messageById(messageId);
    if (current == null || conversationId == null || message == null) {
      return false;
    }
    final comment = feedback?.comment?.trim();
    final next = feedback == null
        ? null
        : MessageFeedback(
            rating: feedback.rating,
            comment: (comment == null || comment.isEmpty) ? null : comment,
          );
    final previous = message.feedback;
    message.feedback = next;
    state = AsyncData(
      current.copyWith(messages: List<ChatMessage>.from(current.messages)),
    );
    ref
        .read(messageFeedbackControllerProvider.notifier)
        .enqueue(
          conversationId: conversationId,
          messageId: messageId,
          previous: previous,
          next: next,
          onFailed: (confirmed) =>
              _restoreFeedback(conversationId, messageId, confirmed),
        );
    return true;
  }

  /// Roll [messageId] back to [confirmed] after a failed call. Does nothing
  /// visible when the user has since left the conversation the call belonged
  /// to.
  void _restoreFeedback(
    String conversationId,
    String messageId,
    MessageFeedback? confirmed,
  ) {
    if (!ref.mounted) return;
    final current = state.value;
    final message = _messageById(messageId);
    if (current == null ||
        current.conversationId != conversationId ||
        message == null) {
      return;
    }
    message.feedback = confirmed;
    state = AsyncData(
      current.copyWith(
        messages: List<ChatMessage>.from(current.messages),
        error: 'Could not save your feedback. Please try again.',
      ),
    );
  }

  /// The user chose [rating] on a message (`null` clears it). A new rating
  /// carries no comment and opens the comment field; clearing closes it.
  void setMessageRating(String messageId, MessageRating? rating) {
    final applied = setMessageFeedback(
      messageId,
      rating == null ? null : MessageFeedback(rating: rating),
    );
    if (!applied) return;
    final feedback = ref.read(messageFeedbackControllerProvider.notifier);
    if (rating == null) {
      feedback.closeCommentField(messageId);
    } else {
      feedback.openCommentField(messageId, rating);
    }
  }

  /// Send [comment] with the message's current rating and close the field.
  /// Ignored while a feedback call for the message is still in flight.
  void submitMessageFeedbackComment(String messageId, String comment) {
    final feedback = ref.read(messageFeedbackControllerProvider.notifier);
    final rating = _messageById(messageId)?.feedback?.rating;
    if (rating == null || feedback.isPending(messageId)) return;
    setMessageFeedback(
      messageId,
      MessageFeedback(rating: rating, comment: comment),
    );
    feedback.closeCommentField(messageId);
  }

  /// Close the comment field without sending anything.
  void dismissMessageFeedbackComment(String messageId) => ref
      .read(messageFeedbackControllerProvider.notifier)
      .closeCommentField(messageId);

  /// Start a fresh conversation (no conversation ID set).
  void clearConversation() {
    conversationChanged();
    _session.clearDisconnected();
    _cancelFollowUps();
    state = const AsyncData(ChatState());
  }

  /// Dismiss the current error without affecting messages or conversation.
  void dismissError() {
    final current = state.value ?? const ChatState();
    state = AsyncData(current.copyWith(clearError: true));
  }

  /// Ask the server to cancel the current run. The terminal frame that follows
  /// reconciles the turn; this call only delivers the request.
  ///
  /// A request the server never answered is retried twice (1 s, 2 s apart)
  /// before detaching with an error; `unknown` (no such run) detaches and
  /// refreshes. Any other answer leaves the outcome to the stream.
  Future<void> _requestServerCancel() async {
    final api = _api;
    final runId = _session.runId;
    final convId = state.value?.conversationId;
    if (api == null || runId == null || convId == null) return;
    final turn = _session.turnSeq;
    for (var attempt = 0; ; attempt++) {
      final result = await api.cancelTurn(convId, runId);
      if (!_stopStillPending(turn)) return;
      if (result == TurnState.unknown) {
        await _detachTurn(refresh: true);
        return;
      }
      if (result != null) return;
      if (attempt == _cancelRetries) {
        await _detachTurn(error: _unreachableStopError);
        return;
      }
      await Future<void>.delayed(ChatStreamSession.retryDelay(attempt));
      if (!_stopStillPending(turn)) return;
    }
  }

  /// Set the active conversation ID without loading history.
  void setConversationId(String id) {
    conversationChanged();
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
      conversationChanged();
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
    _cancelFollowUps();
    final afterCreate = state.value ?? const ChatState();
    state = AsyncData(
      afterCreate.copyWith(
        clearSuggestions: true,
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
    if (!_session.draining) {
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

  /// Edit user message [messageId] to [text] and stream the reply on the new
  /// branch.
  ///
  /// Truncates the timeline after the message, shows [text] as a sending user
  /// bubble that keeps the original attachments, and reloads the detail after
  /// the terminal frame so the server's branch metadata is authoritative.
  /// Refused while a turn works, unless it only waits for input: the server
  /// then cancels that wait. [replaceId] addresses the sending bubble of a
  /// failed edit that has not been persisted yet.
  Future<void> editMessage(
    String messageId,
    String text, {
    String? replaceId,
  }) async {
    await _editTurn(messageId, text, replaceId: replaceId);
  }

  /// Regenerate the reply to [messageId] (a user message or any reply row) and
  /// stream it in place, optionally with another [model].
  ///
  /// Refused while a turn works, unless it only waits for input.
  Future<void> regenerate(String messageId, {String? model}) async {
    await _regenerateTurn(messageId, model: model);
  }

  /// End the in-flight turn locally. Branch operations replace the timeline its
  /// stream would reconcile into, so its late terminal frames are dropped.
  @override
  void supersedeTurn() {
    _session.supersede();
    _stallTimer?.cancel();
    _stallTimer = null;
    _turnInFlight = false;
  }

  Future<_TurnOutcome> _editTurn(
    String messageId,
    String text, {
    String? replaceId,
  }) async {
    final api = _api;
    final current = state.value;
    final conversationId = current?.conversationId;
    if (api == null || current == null || conversationId == null) {
      return _TurnOutcome.refused;
    }
    if (!_branchTurnAllowed(current)) return _TurnOutcome.refused;
    final target = replaceId ?? messageId;
    final idx = current.messages.indexWhere((m) => m.id == target);
    if (idx == -1) return _TurnOutcome.refused;
    final original = current.messages[idx];

    _cancelFollowUps();
    _session.beginTurn();
    final tokenController = StreamController<String>.broadcast();
    _session.openThinking();
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    _session.conversationId = conversationId;
    _session.userMsgId = userMsgId;
    _session.editSource = (messageId: messageId, text: text);

    final edited = ChatMessage(
      id: userMsgId,
      role: 'user',
      content: text,
      status: MessageStatus.sending,
      attachments: List.of(original.attachments),
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
        messages: [...current.messages.take(idx), edited, assistantPlaceholder],
        isSending: true,
        streamingContent: '',
        clearSuggestions: true,
        clearError: true,
      ),
    );
    _beginLifecycle();

    return _streamTurn(
      api: api,
      conversationId: conversationId,
      userMsgId: userMsgId,
      tokenController: tokenController,
      source: api.streamEdit(conversationId, messageId, text),
      reloadOnTerminal: true,
    );
  }

  Future<_TurnOutcome> _regenerateTurn(
    String messageId, {
    String? model,
  }) async {
    final api = _api;
    final current = state.value;
    final conversationId = current?.conversationId;
    if (api == null || current == null || conversationId == null) {
      return _TurnOutcome.refused;
    }
    if (!_branchTurnAllowed(current)) return _TurnOutcome.refused;
    final idx = current.messages.indexWhere((m) => m.id == messageId);
    if (idx == -1) return _TurnOutcome.refused;
    final clicked = current.messages[idx];
    // Keep the user row the reply answers; drop the reply and everything after.
    final cut = clicked.isUser ? idx + 1 : idx;
    final userIdx = current.messages
        .take(cut)
        .toList()
        .lastIndexWhere((m) => m.isUser);
    if (userIdx == -1) return _TurnOutcome.refused;
    final userMessage = current.messages[userIdx];

    _cancelFollowUps();
    _session.beginTurn();
    final tokenController = StreamController<String>.broadcast();
    _session.openThinking();
    _session.conversationId = conversationId;
    _session.userMsgId = userMessage.id;
    _session.serverUserId = userMessage.id;

    final kept = current.messages.take(cut).toList();
    kept[userIdx] = kept[userIdx].copyWith(status: MessageStatus.sending);
    final assistantPlaceholder = ChatMessage(
      id: streamingPlaceholderId,
      role: 'assistant',
      content: '',
      isStreaming: true,
      tokenStream: tokenController.stream,
    );
    state = AsyncData(
      current.copyWith(
        messages: [...kept, assistantPlaceholder],
        isSending: true,
        streamingContent: '',
        clearSuggestions: true,
        clearError: true,
      ),
    );
    _beginLifecycle();

    return _streamTurn(
      api: api,
      conversationId: conversationId,
      userMsgId: userMessage.id,
      tokenController: tokenController,
      source: api.streamRegenerate(conversationId, messageId, model: model),
      reloadOnTerminal: true,
    );
  }

  /// Retry a failed message.
  ///
  /// A failed edit regenerates the edited message's server id and, when that
  /// returns 404, re-issues the edit of the original message — never a send.
  /// Any other failed message with a server id replays its run and, when the
  /// replay does not complete with `done`, regenerates it; a 404 falls back to
  /// re-sending. Without a server id the text is sent again as before.
  Future<void> retryMessage(ChatMessage msg) async {
    final api = _api;
    final current = state.value ?? const ChatState();
    final conversationId = current.conversationId;
    if (api == null) return;

    final edit = _session.editSource;
    if (edit != null && msg.id == _session.userMsgId) {
      if (_session.serverUserId == msg.id && conversationId != null) {
        final outcome = await _regenerateTurn(msg.id);
        if (outcome != _TurnOutcome.notFound) return;
      }
      await _editTurn(edit.messageId, edit.text, replaceId: msg.id);
      return;
    }

    final serverId = _session.serverUserId == msg.id ? msg.id : null;
    if (_session.runId != null && conversationId != null) {
      final replayed = await _replayRun(
        api,
        conversationId,
        msg.id,
        _session.runId!,
        _session.lastSeq,
      );
      if (replayed) return;
    }
    if (serverId != null && conversationId != null) {
      final outcome = await _regenerateTurn(serverId);
      if (outcome != _TurnOutcome.notFound) return;
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
    _session.conversationId = conversationId;
    _session.userMsgId = userMsgId;

    // Put the user message back into sending state and add streaming placeholder.
    final msgs = List<ChatMessage>.from(current.messages);
    setMessageStatus(msgs, userMsgId, MessageStatus.sending);
    // A dropped stream leaves its placeholder behind; reuse it so the replay
    // never ends up with two and reconciliation only touches one.
    final hasPlaceholder = msgs.any((m) => m.id == streamingPlaceholderId);
    if (!hasPlaceholder) {
      msgs.add(
        ChatMessage(
          id: streamingPlaceholderId,
          role: 'assistant',
          content: '',
          isStreaming: true,
        ),
      );
    }
    // A nonzero-cursor replay never repeats `run_started` (it is stored at
    // sequence 0), so restore the snapshot from `runId` before the loop. A
    // retained approval needs it to submit.
    final restoredStatus =
        current.currentTurnStatus ??
        TurnStatusSnapshot(
          runId: runId,
          lastEventKind: TurnEventKind.runStarted,
          lastEventAt: DateTime.now(),
          awaitingLabel: awaitingLabelOf(msgs),
        );
    state = AsyncData(
      current.copyWith(
        messages: msgs,
        isSending: true,
        streamingContent: hasPlaceholder ? current.streamingContent : '',
        clearSuggestions: true,
        currentTurnStatus: restoredStatus,
      ),
    );
    _armStallTimer();

    try {
      await for (final event in _session.untilDetached(
        api.streamEventsFrom(conversationId, runId, since: since),
      )) {
        final chatState = state.value ?? const ChatState();

        if (_applyTurnEvent(chatState, event)) {
          _session.advance(event);
        } else if (event is RunStartedEvent) {
          _captureRunId(event);
        } else if (event is DoneEvent) {
          _session.advance(event);
          _finishTurn(
            chatState,
            event,
            conversationId: conversationId,
            userMsgId: userMsgId,
            source: TurnSource.replay,
          );
          _startFollowUps(
            api,
            event,
            conversationId: conversationId,
            runId: runId,
          );
          return true;
        } else if (event is AgentErrorEvent) {
          _session.advance(event);
          _onAgentError(chatState, event, userMsgId: userMsgId);
          return event.reason == AgentErrorReason.cancelled;
        } else if (event is ErrorEvent) {
          _session.advance(event);
          return false;
        }
      }
      return false;
    } on DioException catch (e) {
      // 404 = run not found, 410 = run expired → caller re-sends.
      final status = e.response?.statusCode;
      if (status == 404 || status == 410) {
        // Restore UI to clean failed state.
        state = AsyncData(
          abortedTurnState(
            state.value ?? const ChatState(),
            failedUserMsgId: userMsgId,
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
  /// A [_session.draining] guard prevents re-entrant calls. The loop continues until
  /// the queue is empty; try/finally ensures the flag is always cleared.
  Future<void> _drainQueue() async {
    if (_session.draining) return;
    _session.draining = true;
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
      _session.draining = false;
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

    // Reset per-turn state so Stop never targets the previous turn's run id.
    _cancelFollowUps();
    _session.beginTurn();
    final current = state.value ?? const ChatState();

    // Create a broadcast stream controller for token-by-token rendering.
    final tokenController = StreamController<String>.broadcast();
    _session.openThinking();

    // Show a placeholder user message while transcribing.
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    _session.conversationId = conversationId;
    _session.userMsgId = userMsgId;
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
        clearSuggestions: true,
      ),
    );
    _beginLifecycle();

    // Arm the same initial-stall watchdog used by the text path. The voice
    // SSE response carries TranscriptEvent + token/status events; any event
    // counts as progress. Until the first event arrives, a quiet interval
    // of [ChatStreamSession.initialStallTimeout] probes the server: a `running` reply keeps
    // the sink open (iOS Dio buffering safety net), anything else recovers.
    var sawProgress = false;
    final source = _session.untilDetached(
      api
          .sendVoiceMessage(conversationId, audioBytes, mimeType)
          .timeout(
            ChatStreamSession.initialStallTimeout,
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
          ),
    );

    try {
      await for (final event in source) {
        // The run id header arrives before the body: not progress.
        if (event is RunStartedEvent) {
          _captureRunId(event);
          _openTurnStatus(event);
          continue;
        }
        sawProgress = true;
        final chatState = state.value ?? const ChatState();

        if (_applyTurnEvent(chatState, event, tokenSink: tokenController)) {
          continue;
        }
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
        } else if (event is DoneEvent) {
          unawaited(tokenController.close());
          _finishTurn(
            chatState,
            event,
            conversationId: conversationId,
            userMsgId: userMsgId,
            source: TurnSource.live,
          );
          return;
        } else if (event is ErrorEvent) {
          unawaited(tokenController.close());
          _failTurn(chatState, event, userMsgId: userMsgId);
          return;
        } else if (event is AgentErrorEvent) {
          unawaited(tokenController.close());
          _onAgentError(chatState, event, userMsgId: userMsgId);
          return;
        }
      }
      // Detached: no terminal frame will come, so release the token stream.
      unawaited(tokenController.close());
    } catch (e) {
      unawaited(tokenController.close());
      await _handleStreamFailure(api, e, conversationId, userMsgId);
    } finally {
      _session.cancelStopTimer();
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

    _cancelFollowUps();
    _session.beginTurn();
    final current = state.value ?? const ChatState();

    // Create broadcast stream controllers for token-by-token rendering.
    final tokenController = StreamController<String>.broadcast();
    _session.openThinking();

    // Add user message with status=sending and assistant streaming placeholder.
    final userMsgId = 'user-${DateTime.now().millisecondsSinceEpoch}';
    _session.conversationId = conversationId;
    _session.userMsgId = userMsgId;
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
        clearSuggestions: true,
      ),
    );
    _beginLifecycle();

    await _streamTurn(
      api: api,
      conversationId: conversationId,
      userMsgId: userMsgId,
      tokenController: tokenController,
      source: api.streamMessages(
        conversationId,
        message,
        attachmentIds: attachmentIds.isNotEmpty ? attachmentIds : null,
      ),
    );
  }

  /// Stream one turn's SSE [source] and reconcile its terminal frame.
  ///
  /// Every non-terminal frame goes through [_applyTurnEvent]; a turn whose
  /// stream ends without a terminal frame keeps what streamed. With
  /// [reloadOnTerminal], the conversation detail is fetched again after `done`
  /// or a cancelled `agent_error`, so the server's branch metadata replaces the
  /// optimistic timeline.
  Future<_TurnOutcome> _streamTurn({
    required ApiClient api,
    required String conversationId,
    required String userMsgId,
    required StreamController<String> tokenController,
    required Stream<StreamEvent> source,
    bool reloadOnTerminal = false,
  }) async {
    final turn = _session.turnSeq;
    var currentUserId = userMsgId;
    var sawProgress = false;
    final wrapped = _session.untilDetached(
      source.timeout(
        ChatStreamSession.initialStallTimeout,
        onTimeout: (sink) {
          if (sawProgress) return;
          unawaited(
            _probeOrRecover(
              conversationId: conversationId,
              userMsgId: currentUserId,
              sink: sink,
              sawProgress: () => sawProgress,
            ),
          );
        },
      ),
    );

    try {
      final terminal = await _session.consume(
        wrapped,
        turn: turn,
        onEvent: (event) {
          if (event is RunStartedEvent) {
            _captureRunId(event);
            currentUserId = _adoptUserMessageId(
              currentUserId,
              event.userMessageId,
            );
            _openTurnStatus(event);
            return;
          }
          sawProgress = true;
          final chatState = state.value ?? const ChatState();
          if (_applyTurnEvent(chatState, event, tokenSink: tokenController)) {
            _session.advance(event);
          }
        },
      );
      unawaited(tokenController.close());
      if (!ref.mounted || !_session.isCurrentTurn(turn)) {
        return _TurnOutcome.superseded;
      }
      if (terminal == null) {
        _finishIncompleteTurn(conversationId, currentUserId);
        return _TurnOutcome.failed;
      }
      _session.advance(terminal);
      final chatState = state.value ?? const ChatState();
      switch (terminal) {
        case DoneEvent done:
          _finishTurn(
            chatState,
            done,
            conversationId: conversationId,
            userMsgId: currentUserId,
            source: TurnSource.live,
          );
          _startFollowUps(
            api,
            done,
            conversationId: conversationId,
            runId: _session.runId,
          );
          if (reloadOnTerminal) await _refreshFromServer(conversationId);
          return _TurnOutcome.done;
        case AgentErrorEvent error:
          _onAgentError(chatState, error, userMsgId: currentUserId);
          if (reloadOnTerminal && error.reason == AgentErrorReason.cancelled) {
            await _refreshFromServer(conversationId);
          }
          return error.reason == AgentErrorReason.cancelled
              ? _TurnOutcome.cancelled
              : _TurnOutcome.failed;
        case ErrorEvent error:
          _failTurn(chatState, error, userMsgId: currentUserId);
          return error.isNotFound ? _TurnOutcome.notFound : _TurnOutcome.failed;
        default:
          return _TurnOutcome.failed;
      }
    } catch (e) {
      unawaited(tokenController.close());
      if (!_session.isCurrentTurn(turn)) return _TurnOutcome.superseded;
      await _handleStreamFailure(api, e, conversationId, currentUserId);
      return _TurnOutcome.failed;
    } finally {
      if (_session.isCurrentTurn(turn)) _session.cancelStopTimer();
    }
  }

  /// Keep what streamed when the server closed the stream without a terminal
  /// frame, and surface an incomplete response as the user message's outcome.
  void _finishIncompleteTurn(String conversationId, String userMsgId) {
    if (!ref.mounted) return;
    _endLifecycle();
    final finalState = state.value ?? const ChatState();
    if (!finalState.isSending) return;
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

  /// Adopt the server's user-row id for the optimistic bubble [from], so a
  /// failed turn can be regenerated by its persisted id. Returns the id the
  /// terminal frame must reconcile.
  String _adoptUserMessageId(String from, String? serverId) {
    if (serverId == null || serverId.isEmpty || serverId == from) return from;
    final current = state.value;
    if (current != null) {
      final idx = current.messages.indexWhere((m) => m.id == from);
      if (idx != -1) {
        final msgs = List<ChatMessage>.from(current.messages);
        msgs[idx] = msgs[idx].copyWith(id: serverId);
        state = AsyncData(current.copyWith(messages: msgs));
      }
    }
    if (_session.userMsgId == from) _session.userMsgId = serverId;
    _session.serverUserId = serverId;
    return serverId;
  }

  /// Whether a branch turn may start: the conversation is idle, or the turn in
  /// flight only waits for the user, whose pending interaction the server
  /// cancels when the branch turn arrives.
  bool _branchTurnAllowed(ChatState state) {
    if (state.isSwitchingBranch) return false;
    if (!state.isSending) return true;
    return state.currentTurnStatus?.awaitingLabel != null;
  }

  /// Recover from an exception that ended a turn's event stream.
  ///
  /// A transient error with a run ID retries the event-log replay with
  /// exponential backoff, then defers to app-resume reconnection if every
  /// retry fails. Anything else fails the turn immediately.
  Future<void> _handleStreamFailure(
    ApiClient api,
    Object error,
    String conversationId,
    String userMsgId,
  ) async {
    _session.closeThinking();
    if (!ref.mounted) return;

    if (isTransientStreamError(error) && _session.runId != null) {
      for (
        var attempt = 0;
        attempt < ChatStreamSession.maxStreamRetries;
        attempt++
      ) {
        await Future<void>.delayed(ChatStreamSession.retryDelay(attempt));
        if (!ref.mounted || _session.detached) return;
        if (_session.stopRequested) {
          // Stopped while the stream was down: the cancel was sent, so do not
          // replay again; keep what streamed so far as the stopped reply.
          state = AsyncData(
            stoppedTurnState(
              state.value ?? const ChatState(),
              userMsgId: userMsgId,
              partialContent: '',
            ),
          );
          _endLifecycle((at) => TurnStopped(at: at));
          return;
        }

        final replayed = await _replayRun(
          api,
          conversationId,
          userMsgId,
          _session.runId!,
          _session.lastSeq,
        );
        if (replayed || _session.terminalFrameSeen) return;
      }

      // All retries exhausted — defer to app resume. The turn stays tracked,
      // so a replay on resume is announced, but it is not stalled.
      _stallTimer?.cancel();
      _session.markDisconnected(
        userMsgId: userMsgId,
        conversationId: conversationId,
      );
      state = AsyncData(
        abortedTurnState(
          state.value ?? const ChatState(),
          clearStatusMessage: true,
          clearTurnStatus: true,
        ),
      );
      return;
    }

    // Non-transient error or no run ID: show error immediately.
    state = AsyncData(
      abortedTurnState(
        state.value ?? const ChatState(),
        failedUserMsgId: userMsgId,
        error: _stoppedBeforeRunId ? null : friendlyStreamErrorMessage(error),
        clearTurnStatus: true,
      ),
    );
    _endLifecycle(
      _stoppedBeforeRunId
          ? (at) => TurnStopped(at: at)
          : (at) =>
                TurnFailed(message: friendlyStreamErrorMessage(error), at: at),
    );
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
    if (sawProgress() || _session.detached) return;
    final api = _api;
    final runId = _session.runId;
    if (api == null || runId == null) {
      // RunStarted hasn't fired yet (or no client) — no run id to probe.
      // Fall back to the legacy recovery path so the dots state still
      // resolves within the watchdog window.
      await _recoverStalledStream(conversationId, userMsgId);
      if (!_session.detached) sink.close();
      return;
    }
    final state = await api.turnStatus(conversationId, runId);
    if (sawProgress() || _session.detached) return;
    if (_isLive(state)) {
      // Server says the turn is still in flight (or waiting for the owner to
      // answer an approval) — leave the sink open.
      // The chat-stream-progress-ux card already shows a stalled UI past
      // `kTurnStallThreshold`; the 90 s byte-watchdog is the final
      // safety net for genuinely dead connections.
      return;
    }
    // completed / errored / unknown / null (probe failed) → recover.
    await _recoverStalledStream(conversationId, userMsgId);
    if (!_session.detached) sink.close();
  }

  /// Recover from a stalled SSE stream by fetching the conversation
  /// directly. Triggered by the [ChatStreamSession.initialStallTimeout] watchdog when the
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
    if (_session.detached || !ref.mounted) return;
    _session.detach(); // end the stream loop at once
    _endLifecycle();

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
    await _refreshFromServer(conversationId);
  }

  /// Replace the messages with the server's persisted ones, keeping
  /// [ChatState.pendingQueue]. `loadConversation` is not used because it
  /// builds a fresh state and drops the queue. On failure the state is left
  /// as it is.
  Future<void> _refreshFromServer(String conversationId) async {
    final api = _api;
    if (api == null) return;
    try {
      final response = await api.conversations.getConversation(
        id: conversationId,
      );
      if (!ref.mounted) return;
      final messages = _withPendingFeedback(
        chatMessagesFromHistory(response.data!.messages),
      );
      final latest = state.value ?? const ChatState();
      // While we were awaiting the conversation fetch, the queue-drain
      // loop may have advanced to the next pending message and started
      // a new stream. Overwriting state here would silently wipe that
      // in-flight placeholder. The new stream is the freshest truth —
      // discard the stale snapshot instead.
      //
      // Reproducer / regression test:
      //   test/unit/chat/chat_provider_test.dart — "stopTurn — bounded wait"
      //   "cancel unknown detaches and refreshes".
      if (latest.isSending) return;
      state = AsyncData(
        ChatState(
          conversationId: conversationId,
          messages: messages,
          pendingQueue: latest.pendingQueue,
        ),
      );
    } catch (_) {
      // Fetch failed — keep the current state (and any error already shown)
      // so the user can retry.
    }
  }

  void _clearStalledPlaceholder(
    ChatState from,
    String userMsgId, {
    required String error,
  }) {
    if (!ref.mounted) return;
    state = AsyncData(
      abortedTurnState(
        from,
        failedUserMsgId: userMsgId,
        error: error,
        clearTurnStatus: true,
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
    final disconnected = _session.takeDisconnected();
    if (disconnected == null) {
      // No interrupted stream to recover, but still kick the drain if
      // there are queued messages and nothing is processing them. This
      // is a defence-in-depth net for races elsewhere in the state
      // machine.
      if (!_session.draining &&
          (state.value?.pendingQueue.isNotEmpty ?? false)) {
        unawaited(_drainQueue());
      }
      return;
    }

    final api = _api;
    final conversationId = disconnected.conversationId;
    final userMsgId = disconnected.userMsgId;
    final runId = _session.runId;

    if (api == null) return;

    // Surface the recovery work as a banner — set BEFORE any await so
    // the UI shows "Reconnecting…" promptly. Cleared in every exit path.
    final current = state.value ?? const ChatState();
    state = AsyncData(current.copyWith(isReconnecting: true));

    try {
      // Strategy 1: Replay from last event sequence.
      if (runId != null) {
        try {
          final replayed = await _replayRun(
            api,
            conversationId,
            userMsgId,
            runId,
            _session.lastSeq,
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
      setMessageStatus(msgs, userMsgId, MessageStatus.failed);
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
}

/// How a branching turn ended, for [ChatNotifier.retryMessage]'s fallbacks.
enum _TurnOutcome {
  /// The server streamed a `done` frame.
  done,

  /// The turn was cancelled; the reload shows what the server persisted.
  cancelled,

  /// The turn failed, or its stream ended without a terminal frame.
  failed,

  /// The request was rejected with 404: the message was never persisted.
  notFound,

  /// A newer turn took over; this stream must not touch the timeline.
  superseded,

  /// The turn never started, because a turn works or the target is gone.
  refused,
}

/// Provider for [ChatNotifier].
final chatProvider = AsyncNotifierProvider.autoDispose<ChatNotifier, ChatState>(
  ChatNotifier.new,
);
