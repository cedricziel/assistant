import 'dart:async';

import '../../api/models/stream_event.dart';

/// A turn interrupted by a transient connection error, waiting for the app to
/// resume so it can be recovered.
typedef DisconnectedTurn = ({String conversationId, String userMsgId});

/// Mutable bookkeeping for the chat turn currently streaming (or last
/// streamed) by `ChatNotifier`: stop and detach, the replay cursor, deferred
/// reconnection and the thinking token stream.
class ChatStreamSession {
  /// Maximum number of replay retries with exponential backoff before
  /// deferring to app-resume reconnection.
  static const maxStreamRetries = 3;

  /// How long to wait after Send for the first non-`RunStartedEvent` from
  /// the SSE stream before treating the stream as stalled and falling back
  /// to a direct conversation fetch.
  ///
  /// Motivated by iOS Dio / dart:io HttpClient buffering: the response
  /// headers arrive (so `RunStartedEvent` fires from the `X-Run-Id`
  /// header) but no body chunks reach the parser until the connection
  /// closes, leaving the user staring at "jumpy dots" until the byte-
  /// level 90 s heartbeat finally trips. 12 s is short enough to feel
  /// responsive and long enough to absorb a brief first-token delay.
  static const initialStallTimeout = Duration(seconds: 12);

  /// How long Stop waits for the server to confirm, counted from the tap,
  /// before the client detaches locally.
  static const stopGrace = Duration(seconds: 10);

  static const _baseRetryDelay = Duration(seconds: 1);

  /// Backoff before retry [attempt] (0-based): 1s, 2s, 4s.
  static Duration retryDelay(int attempt) => _baseRetryDelay * (1 << attempt);

  Completer<void> _detach = Completer<void>();

  /// Bumped by [beginTurn]; lets a late async continuation tell that its turn
  /// is over.
  int turnSeq = 0;

  /// The user message and conversation of the turn in flight, so a local
  /// detach can reconcile it without the stream loop.
  String? userMsgId;
  String? conversationId;

  /// Set once the server's terminal `agent_error` frame was reconciled, so a
  /// failed replay attempt does not replay a finished turn again.
  bool terminalFrameSeen = false;

  /// Fires [stopGrace] after Stop was tapped.
  Timer? stopTimer;

  /// Set when the user taps Stop; stays set for the rest of the turn. While
  /// the run id is still unknown it defers the server cancel to
  /// `RunStartedEvent`.
  bool stopRequested = false;

  /// Whether the pending-queue drain loop is running.
  bool draining = false;

  /// Run ID of the most recent (or current) orchestrator run, captured from
  /// the `RunStartedEvent` so the stream can be replayed if it drops.
  String? runId;

  /// Replay cursor: the `since` value to request from the event log.
  int lastSeq = 0;

  DisconnectedTurn? _disconnected;
  StreamController<String>? _thinking;

  /// Whether a transient interruption should be retried on app resume.
  bool get needsReconnect => _disconnected != null;

  /// The open thinking token stream, if any.
  StreamController<String>? get thinking => _thinking;

  /// Whether the turn was detached locally (a Stop that the server did not
  /// confirm in time, or a stall recovery). Stream loops end at once.
  bool get detached => _detach.isCompleted;

  /// Detach from the server's stream: [untilDetached] sources close without
  /// waiting for another server event.
  void detach() {
    cancelStopTimer();
    if (!_detach.isCompleted) _detach.complete();
  }

  /// Stop the [stopGrace] timer, if running.
  void cancelStopTimer() {
    stopTimer?.cancel();
    stopTimer = null;
  }

  /// Forward [source] until [detach] is called, then close and drop it.
  Stream<T> untilDetached<T>(Stream<T> source) {
    final detachFuture = _detach.future;
    late final StreamController<T> controller;
    StreamSubscription<T>? subscription;
    controller = StreamController<T>(
      onListen: () {
        subscription = source.listen(
          controller.add,
          onError: controller.addError,
          onDone: controller.close,
        );
        unawaited(
          detachFuture.then((_) {
            unawaited(subscription?.cancel());
            if (!controller.isClosed) unawaited(controller.close());
          }),
        );
      },
      onPause: () => subscription?.pause(),
      onResume: () => subscription?.resume(),
      // Never wait for the source to finish cancelling: a stalled
      // connection must not hold up the turn that is being detached.
      onCancel: () async {
        unawaited(subscription?.cancel());
      },
    );
    return controller.stream;
  }

  /// Reset per-turn state before streaming a new message.
  void beginTurn() {
    cancelStopTimer();
    _detach = Completer<void>();
    turnSeq++;
    terminalFrameSeen = false;
    userMsgId = null;
    conversationId = null;
    stopRequested = false;
    runId = null;
    lastSeq = 0;
    clearDisconnected();
  }

  /// Advance the replay cursor to one past [event].
  void advance(StreamEvent event) {
    lastSeq = (event.sequenceId ?? lastSeq) + 1;
  }

  /// Remember the interrupted turn so [takeDisconnected] can recover it.
  void markDisconnected({
    required String userMsgId,
    required String conversationId,
  }) {
    _disconnected = (conversationId: conversationId, userMsgId: userMsgId);
  }

  /// Return and clear the interrupted turn; `null` when nothing is deferred.
  DisconnectedTurn? takeDisconnected() {
    final taken = _disconnected;
    _disconnected = null;
    return taken;
  }

  /// Drop any deferred reconnection.
  void clearDisconnected() => _disconnected = null;

  /// Open a fresh broadcast stream for incremental thinking rendering.
  StreamController<String> openThinking() {
    closeThinking();
    return _thinking = StreamController<String>.broadcast();
  }

  /// Close and forget the thinking stream.
  void closeThinking() {
    final c = _thinking;
    _thinking = null;
    if (c != null) unawaited(c.close());
  }
}
