import 'chat_provider.dart' show TurnEventKind, TurnStatusSnapshot;

/// Returns a plain-language label describing the active turn's current
/// activity, derived from the most recent SSE event.
///
/// Pure function — no `BuildContext`, no localisation hooks (yet). Returns
/// the English copy expected by the in-flight progress card. When i18n is
/// added later, the lookup moves through `AppLocalizations`.
///
/// Specified by `openspec/changes/chat-stream-progress-ux/specs/...`
/// "In-flight turn progress card" requirement.
String turnStatusLabel(TurnStatusSnapshot snapshot) {
  switch (snapshot.lastEventKind) {
    case TurnEventKind.runStarted:
      return 'Starting…';
    case TurnEventKind.thinking:
      return 'Thinking…';
    case TurnEventKind.token:
      return 'Generating response…';
    case TurnEventKind.status:
      return 'Working…';
    case TurnEventKind.toolResult:
      final name = snapshot.currentToolName;
      if (name != null && name.isNotEmpty) {
        return 'Calling $name…';
      }
      return 'Running tool…';
    case TurnEventKind.subagentStarted:
      final task = snapshot.currentToolName;
      if (task != null && task.isNotEmpty) {
        return 'Subagent: $task…';
      }
      return 'Running subagent…';
    case TurnEventKind.subagentCompleted:
      return 'Subagent finished — generating…';
    case TurnEventKind.audioReady:
      return 'Audio ready';
  }
}

/// Returns the stall-state label rendered when the active turn has been
/// silent for longer than the stall threshold. Includes a compact elapsed-
/// time string in the form `0:34`.
///
/// Callers pass the elapsed-seconds count from a [Ticker] in the widget.
String stalledTurnStatusLabel(Duration elapsed) {
  final minutes = elapsed.inMinutes;
  final seconds = elapsed.inSeconds % 60;
  final compact = '$minutes:${seconds.toString().padLeft(2, '0')}';
  return 'Server is taking longer than expected — $compact';
}
