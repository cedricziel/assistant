// Pure unit tests for the `turnStatusLabel` and `stalledTurnStatusLabel`
// helpers. Locks the user-facing copy per `TurnEventKind`, the tool-name
// interpolation, and the stalled elapsed-time format.
//
// Specified by: openspec/changes/chat-stream-progress-ux/specs/...

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/turn_status_label.dart';

TurnStatusSnapshot _snap(TurnEventKind kind, {String? toolName}) =>
    TurnStatusSnapshot(
      runId: 'run-1',
      lastEventKind: kind,
      lastEventAt: DateTime(2026, 5, 17, 12, 0, 0),
      currentToolName: toolName,
    );

void main() {
  group('turnStatusLabel', () {
    test('runStarted → "Starting…"', () {
      expect(turnStatusLabel(_snap(TurnEventKind.runStarted)), 'Starting…');
    });

    test('thinking → "Thinking…"', () {
      expect(turnStatusLabel(_snap(TurnEventKind.thinking)), 'Thinking…');
    });

    test('token → "Generating response…"', () {
      expect(
        turnStatusLabel(_snap(TurnEventKind.token)),
        'Generating response…',
      );
    });

    test('status → "Working…"', () {
      expect(turnStatusLabel(_snap(TurnEventKind.status)), 'Working…');
    });

    test('toolResult with tool name → "Calling {name}…"', () {
      expect(
        turnStatusLabel(_snap(TurnEventKind.toolResult, toolName: 'fetch_url')),
        'Calling fetch_url…',
      );
    });

    test('toolResult without tool name → generic fallback', () {
      expect(
        turnStatusLabel(_snap(TurnEventKind.toolResult)),
        'Running tool…',
        reason:
            'When the server omits the tool name, the label should still be '
            'meaningful — not just "Calling …"',
      );
    });

    test('subagentStarted with task → "Subagent: {task}…"', () {
      expect(
        turnStatusLabel(
          _snap(TurnEventKind.subagentStarted, toolName: 'analyze data'),
        ),
        'Subagent: analyze data…',
      );
    });

    test('subagentStarted without task → generic fallback', () {
      expect(
        turnStatusLabel(_snap(TurnEventKind.subagentStarted)),
        'Running subagent…',
      );
    });

    test('subagentCompleted → "Subagent finished — generating…"', () {
      expect(
        turnStatusLabel(_snap(TurnEventKind.subagentCompleted)),
        'Subagent finished — generating…',
      );
    });

    test('audioReady → "Audio ready"', () {
      expect(turnStatusLabel(_snap(TurnEventKind.audioReady)), 'Audio ready');
    });
  });

  group('stalledTurnStatusLabel', () {
    test('formats sub-minute elapsed as "0:SS"', () {
      expect(
        stalledTurnStatusLabel(const Duration(seconds: 34)),
        'Server is taking longer than expected — 0:34',
      );
    });

    test('formats elapsed past 1 minute as "M:SS"', () {
      expect(
        stalledTurnStatusLabel(const Duration(seconds: 95)),
        'Server is taking longer than expected — 1:35',
      );
    });

    test('pads single-digit seconds with a leading zero', () {
      expect(
        stalledTurnStatusLabel(const Duration(seconds: 9)),
        'Server is taking longer than expected — 0:09',
      );
    });

    test('handles exactly N minutes (00 seconds)', () {
      expect(
        stalledTurnStatusLabel(const Duration(minutes: 2)),
        'Server is taking longer than expected — 2:00',
      );
    });
  });
}
