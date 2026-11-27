/// Plain-language summary of which SSE event last drove the in-flight turn.
/// Drives the UI's progress card. Add a value here if the server protocol
/// grows a new event kind that the UI should distinguish.
enum TurnEventKind {
  runStarted,
  token,
  status,
  thinking,
  toolResult,
  subagentStarted,
  subagentCompleted,
  audioReady,
}

/// Snapshot of the in-flight turn's most recent activity, exposed for UI
/// rendering of progress indicators. `null` on [ChatState.currentTurnStatus]
/// means no turn is in flight.
///
/// Updated by [ChatNotifier] on each SSE event within `_streamMessage`.
/// Cleared on `DoneEvent`, on the catch block after retries exhaust, and
/// in `_recoverStalledStream`'s synchronous prefix.
class TurnStatusSnapshot {
  const TurnStatusSnapshot({
    required this.runId,
    required this.lastEventKind,
    required this.lastEventAt,
    this.currentToolName,
    this.awaitingLabel,
  });

  /// The run ID of the active turn — server-issued, captured from
  /// `RunStartedEvent`.
  final String runId;

  /// Which kind of SSE event last fired for this turn.
  final TurnEventKind lastEventKind;

  /// Wall-clock timestamp of the most recent SSE event. The widget
  /// derives "seconds since last event" via a [Ticker].
  final DateTime lastEventAt;

  /// Name of the tool or subagent task currently being executed, if
  /// the most recent event implied one. Cleared when the assistant
  /// resumes producing text (`TokenEvent`).
  final String? currentToolName;

  /// What the turn waits for while an interaction is pending, such as
  /// "Waiting for your approval". Null otherwise. There is deliberately no
  /// [TurnEventKind] for it: the progress card swaps its label instead.
  final String? awaitingLabel;

  TurnStatusSnapshot copyWith({
    String? runId,
    TurnEventKind? lastEventKind,
    DateTime? lastEventAt,
    String? currentToolName,
    bool clearToolName = false,
    String? awaitingLabel,
    bool clearAwaitingLabel = false,
  }) {
    return TurnStatusSnapshot(
      runId: runId ?? this.runId,
      lastEventKind: lastEventKind ?? this.lastEventKind,
      lastEventAt: lastEventAt ?? this.lastEventAt,
      currentToolName: clearToolName
          ? null
          : (currentToolName ?? this.currentToolName),
      awaitingLabel: clearAwaitingLabel
          ? null
          : (awaitingLabel ?? this.awaitingLabel),
    );
  }
}
