// Widget tests for [TurnProgressCard], pumped with literal arguments and no
// ProviderScope: the card takes its status, clock and Skip affordance by
// parameter and calls no API.

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

final _now = DateTime.utc(2026, 1, 15, 12);

TurnStatusSnapshot _snap(
  TurnEventKind kind, {
  String? toolName,
  Duration age = Duration.zero,
}) => TurnStatusSnapshot(
  runId: 'run-1',
  lastEventKind: kind,
  lastEventAt: _now.subtract(age),
  currentToolName: toolName,
);

Future<void> _pump(
  WidgetTester tester, {
  TurnStatusSnapshot? status,
  bool showSkip = true,
  VoidCallback? onSkip,
  DateTime Function()? now,
  String? awaitingLabel,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: TurnProgressCard(
        status: status,
        showSkip: showSkip,
        onSkip: onSkip ?? () {},
        now: now ?? () => _now,
        awaitingLabel: awaitingLabel,
      ),
    ),
  ),
);

const _label = Key('turn_progress_card_label');
const _skip = Key('turn_progress_card_skip_button');

void main() {
  group('TurnProgressCard', () {
    testWidgets('renders nothing when status is null', (tester) async {
      await _pump(tester);

      expect(find.byKey(_label), findsNothing);
    });

    testWidgets('renders the activity label for a runStarted status', (
      tester,
    ) async {
      await _pump(tester, status: _snap(TurnEventKind.runStarted));

      expect(find.text('Starting…'), findsOneWidget);
    });

    testWidgets('renders a tool-name label for toolResult', (tester) async {
      await _pump(
        tester,
        status: _snap(TurnEventKind.toolResult, toolName: 'fetch_url'),
      );

      expect(find.text('Calling fetch_url…'), findsOneWidget);
    });

    testWidgets('renders a subagent-task label for subagentStarted', (
      tester,
    ) async {
      await _pump(
        tester,
        status: _snap(TurnEventKind.subagentStarted, toolName: 'crunch'),
      );

      expect(find.text('Subagent: crunch…'), findsOneWidget);
    });

    testWidgets('renders the stall label once the status is stale', (
      tester,
    ) async {
      await _pump(
        tester,
        status: _snap(TurnEventKind.token, age: const Duration(seconds: 35)),
      );

      expect(
        find.text('Server is taking longer than expected — 0:35'),
        findsOneWidget,
      );
      expect(find.text('Generating response…'), findsNothing);
    });

    testWidgets('a status exactly at the threshold is stalled', (tester) async {
      await _pump(
        tester,
        status: _snap(TurnEventKind.token, age: kTurnStallThreshold),
      );

      expect(
        find.textContaining('Server is taking longer than expected'),
        findsOneWidget,
      );
    });

    testWidgets('returns to the activity label when a fresher status arrives', (
      tester,
    ) async {
      await _pump(
        tester,
        status: _snap(TurnEventKind.token, age: const Duration(seconds: 35)),
      );
      expect(find.textContaining('Server is taking longer'), findsOneWidget);

      await _pump(tester, status: _snap(TurnEventKind.thinking));

      expect(find.textContaining('Server is taking longer'), findsNothing);
      expect(find.text('Thinking…'), findsOneWidget);
    });

    testWidgets('disappears when the status is cleared', (tester) async {
      await _pump(tester, status: _snap(TurnEventKind.runStarted));
      expect(find.text('Starting…'), findsOneWidget);

      await _pump(tester);

      expect(find.byKey(_label), findsNothing);
    });

    testWidgets('re-reads the clock every second and crosses into stall', (
      tester,
    ) async {
      var clock = _now;
      await _pump(tester, status: _snap(TurnEventKind.token), now: () => clock);
      expect(find.text('Generating response…'), findsOneWidget);

      clock = _now.add(kTurnStallThreshold);
      await tester.pump(const Duration(seconds: 1));

      expect(
        find.text('Server is taking longer than expected — 0:30'),
        findsOneWidget,
      );
    });

    testWidgets('keeps no timer running once the status is null', (
      tester,
    ) async {
      await _pump(tester, status: _snap(TurnEventKind.runStarted));
      await _pump(tester);

      // A leaked periodic timer fails the test's pending-timer check.
      await tester.pump(const Duration(seconds: 5));
    });

    testWidgets('keeps no timer running after removal', (tester) async {
      await _pump(tester, status: _snap(TurnEventKind.runStarted));
      await tester.pumpWidget(const SizedBox());

      await tester.pump(const Duration(seconds: 5));
    });

    group('awaitingLabel', () {
      testWidgets('replaces the activity label', (tester) async {
        await _pump(
          tester,
          status: _snap(TurnEventKind.toolResult, toolName: 'bash'),
          awaitingLabel: 'Waiting for your approval',
        );

        expect(find.text('Waiting for your approval'), findsOneWidget);
        expect(find.text('Calling bash…'), findsNothing);
      });

      testWidgets('hides the stall label and Skip after long silence', (
        tester,
      ) async {
        await _pump(
          tester,
          status: _snap(
            TurnEventKind.toolResult,
            age: const Duration(minutes: 5),
          ),
          awaitingLabel: 'Waiting for your approval',
        );

        expect(find.text('Waiting for your approval'), findsOneWidget);
        expect(find.textContaining('taking longer'), findsNothing);
        expect(find.byKey(_skip), findsNothing);
      });

      testWidgets('the stall label returns once the label is cleared', (
        tester,
      ) async {
        await _pump(
          tester,
          status: _snap(TurnEventKind.token, age: const Duration(seconds: 35)),
        );

        expect(find.textContaining('taking longer'), findsOneWidget);
      });
    });

    group('Skip', () {
      testWidgets('is hidden while the card shows an activity label', (
        tester,
      ) async {
        await _pump(tester, status: _snap(TurnEventKind.runStarted));

        expect(find.byKey(_skip), findsNothing);
      });

      testWidgets('appears once stalled', (tester) async {
        await _pump(
          tester,
          status: _snap(TurnEventKind.token, age: const Duration(seconds: 35)),
        );

        expect(find.byKey(_skip), findsOneWidget);
        expect(find.text('Skip'), findsOneWidget);
      });

      testWidgets('is hidden when showSkip is false', (tester) async {
        await _pump(
          tester,
          showSkip: false,
          status: _snap(TurnEventKind.token, age: const Duration(seconds: 35)),
        );

        expect(find.byKey(_skip), findsNothing);
      });

      testWidgets('tapping it fires onSkip exactly once', (tester) async {
        var skips = 0;
        await _pump(
          tester,
          onSkip: () => skips++,
          status: _snap(TurnEventKind.token, age: const Duration(seconds: 35)),
        );

        await tester.tap(find.byKey(_skip));
        await tester.pump();

        expect(skips, 1);
      });
    });
  });
}
