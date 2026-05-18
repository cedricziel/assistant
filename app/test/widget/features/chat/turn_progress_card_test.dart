// Widget tests for [TurnProgressCard]. Pins:
//   - rendering nothing when no turn is in flight
//   - rendering the correct label for each event kind
//   - transitioning to the stalled label after kTurnStallThreshold
//   - cleaning up the ticker on disposal
//
// Specified by: openspec/changes/chat-stream-progress-ux/specs/...
//   "In-flight turn progress card"

import 'package:assistant_api/assistant_api.dart';
import 'package:fake_async/fake_async.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/turn_progress_card.dart';

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);
}

/// A notifier that overrides [requestCancelTurn] to count invocations
/// without performing any network I/O. Used by the Skip button widget
/// test.
class _SpyChatNotifier extends ChatNotifier {
  int cancelCalls = 0;

  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);

  @override
  Future<TurnState?> requestCancelTurn() async {
    cancelCalls++;
    return TurnState.errored;
  }
}

Future<_FakeChatNotifier> _pumpCard(WidgetTester tester) async {
  final notifier = _FakeChatNotifier();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [chatProvider.overrideWith(() => notifier)],
      child: const MaterialApp(home: Scaffold(body: TurnProgressCard())),
    ),
  );
  return notifier;
}

TurnStatusSnapshot _snap(
  TurnEventKind kind, {
  String? toolName,
  DateTime? at,
}) => TurnStatusSnapshot(
  runId: 'run-1',
  lastEventKind: kind,
  lastEventAt: at ?? DateTime.now(),
  currentToolName: toolName,
);

void main() {
  group('TurnProgressCard', () {
    testWidgets('renders nothing when no turn is in flight', (tester) async {
      await _pumpCard(tester);
      await tester.pump();

      // No label widget — the card collapses to SizedBox.shrink().
      expect(find.byKey(const Key('turn_progress_card_label')), findsNothing);
    });

    testWidgets('renders the activity label for a runStarted snapshot', (
      tester,
    ) async {
      final notifier = await _pumpCard(tester);
      notifier.push(
        ChatState(currentTurnStatus: _snap(TurnEventKind.runStarted)),
      );
      await tester.pump();

      expect(find.text('Starting…'), findsOneWidget);
    });

    testWidgets('renders a tool-name label for toolResult', (tester) async {
      final notifier = await _pumpCard(tester);
      notifier.push(
        ChatState(
          currentTurnStatus: _snap(
            TurnEventKind.toolResult,
            toolName: 'fetch_url',
          ),
        ),
      );
      await tester.pump();

      expect(find.text('Calling fetch_url…'), findsOneWidget);
    });

    testWidgets('renders a subagent-task label for subagentStarted', (
      tester,
    ) async {
      final notifier = await _pumpCard(tester);
      notifier.push(
        ChatState(
          currentTurnStatus: _snap(
            TurnEventKind.subagentStarted,
            toolName: 'crunch numbers',
          ),
        ),
      );
      await tester.pump();

      expect(find.text('Subagent: crunch numbers…'), findsOneWidget);
    });

    testWidgets('transitions to stalled label after stall threshold', (
      tester,
    ) async {
      final notifier = await _pumpCard(tester);

      // Push a snapshot whose lastEventAt is 35 s in the past — already
      // past the 30 s stall threshold.
      final past = DateTime.now().subtract(const Duration(seconds: 35));
      notifier.push(
        ChatState(currentTurnStatus: _snap(TurnEventKind.token, at: past)),
      );
      await tester.pump();

      // The card should be showing the stall label, not the activity
      // label. The exact elapsed seconds depends on wall-clock timing
      // between push and pump, so match on the stall prefix.
      expect(
        find.textContaining('Server is taking longer than expected'),
        findsOneWidget,
      );
      expect(find.text('Generating response…'), findsNothing);
    });

    testWidgets(
      'transitions back to activity label when a fresher event arrives',
      (tester) async {
        final notifier = await _pumpCard(tester);
        // Start stalled (35 s old).
        notifier.push(
          ChatState(
            currentTurnStatus: _snap(
              TurnEventKind.token,
              at: DateTime.now().subtract(const Duration(seconds: 35)),
            ),
          ),
        );
        await tester.pump();
        expect(
          find.textContaining('Server is taking longer than expected'),
          findsOneWidget,
        );

        // A fresh event arrives — lastEventAt updates to now.
        notifier.push(
          ChatState(
            currentTurnStatus: _snap(
              TurnEventKind.thinking,
              at: DateTime.now(),
            ),
          ),
        );
        await tester.pump();

        // Stall label gone; activity label visible.
        expect(
          find.textContaining('Server is taking longer than expected'),
          findsNothing,
        );
        expect(find.text('Thinking…'), findsOneWidget);
      },
    );

    testWidgets('disappears when currentTurnStatus is cleared', (tester) async {
      final notifier = await _pumpCard(tester);
      notifier.push(
        ChatState(currentTurnStatus: _snap(TurnEventKind.runStarted)),
      );
      await tester.pump();
      expect(find.text('Starting…'), findsOneWidget);

      notifier.push(const ChatState());
      await tester.pump();
      expect(find.byKey(const Key('turn_progress_card_label')), findsNothing);
    });

    testWidgets('ticker fires once per second to update elapsed-time display', (
      tester,
    ) async {
      final notifier = await _pumpCard(tester);

      // Push a snapshot whose lastEventAt is right at the stall
      // threshold, so the very next tick crosses into stall state.
      final atThreshold = DateTime.now().subtract(const Duration(seconds: 30));
      notifier.push(
        ChatState(
          currentTurnStatus: _snap(TurnEventKind.token, at: atThreshold),
        ),
      );
      await tester.pump();

      // After one second of real time, the ticker fires setState
      // and the label updates with the new elapsed-time count. We
      // verify the card stays in stall state (the label may still
      // be either pre-30s threshold or post; what we pin is the
      // periodic Timer keeps it rendering without errors).
      await tester.pump(const Duration(seconds: 1));
      await tester.pump(const Duration(seconds: 1));
      expect(
        find.byKey(const Key('turn_progress_card_label')),
        findsOneWidget,
        reason:
            'The card must keep rendering across ticker frames — no '
            'dispose-during-build errors or null-state crashes',
      );
    });

    // -- Skip button (gated by kSkipButtonEnabled) ---------------------------

    testWidgets(
      'Skip button is hidden while the card is in the activity state',
      (tester) async {
        final notifier = await _pumpCard(tester);
        // Not stalled.
        notifier.push(
          ChatState(currentTurnStatus: _snap(TurnEventKind.runStarted)),
        );
        await tester.pump();
        expect(
          find.byKey(const Key('turn_progress_card_skip_button')),
          findsNothing,
          reason:
              'The Skip affordance must only appear once the card has '
              'transitioned to the stalled state; otherwise users could '
              'cancel a healthy turn mid-progress',
        );
      },
    );

    testWidgets('Skip button appears once the card transitions to stalled', (
      tester,
    ) async {
      final notifier = await _pumpCard(tester);
      // 35 s in the past → past the 30 s stall threshold.
      final past = DateTime.now().subtract(const Duration(seconds: 35));
      notifier.push(
        ChatState(currentTurnStatus: _snap(TurnEventKind.token, at: past)),
      );
      await tester.pump();

      expect(
        find.byKey(const Key('turn_progress_card_skip_button')),
        findsOneWidget,
      );
      expect(find.text('Skip'), findsOneWidget);
    });

    testWidgets('Tapping Skip invokes requestCancelTurn on the notifier', (
      tester,
    ) async {
      // Spy on requestCancelTurn by counting invocations.
      final spy = _SpyChatNotifier();
      await tester.pumpWidget(
        ProviderScope(
          overrides: [chatProvider.overrideWith(() => spy)],
          child: const MaterialApp(home: Scaffold(body: TurnProgressCard())),
        ),
      );

      final past = DateTime.now().subtract(const Duration(seconds: 35));
      spy.push(
        ChatState(currentTurnStatus: _snap(TurnEventKind.token, at: past)),
      );
      await tester.pump();

      await tester.tap(find.byKey(const Key('turn_progress_card_skip_button')));
      await tester.pump();

      expect(
        spy.cancelCalls,
        1,
        reason: 'Tapping Skip must call requestCancelTurn exactly once',
      );
    });

    testWidgets('disposes ticker cleanly on widget removal', (tester) async {
      FakeAsync().run((async) {
        final notifier = _FakeChatNotifier();
        runApp(
          ProviderScope(
            overrides: [chatProvider.overrideWith(() => notifier)],
            child: const MaterialApp(home: Scaffold(body: TurnProgressCard())),
          ),
        );
        notifier.push(
          ChatState(currentTurnStatus: _snap(TurnEventKind.runStarted)),
        );
        async.elapse(const Duration(seconds: 1));
        // Pump removal — replace with an empty scaffold.
        runApp(
          ProviderScope(
            overrides: [chatProvider.overrideWith(() => notifier)],
            child: const MaterialApp(home: Scaffold()),
          ),
        );
        async.elapse(const Duration(seconds: 5));
        // No pending timers means the ticker was cancelled in dispose.
        expect(
          async.pendingTimers,
          isEmpty,
          reason:
              'TurnProgressCard.dispose() must cancel its 1-second '
              'periodic ticker',
        );
      });
    });
  });
}
