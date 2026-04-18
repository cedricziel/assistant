import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/traces/traces_provider.dart';
import 'package:assistant_app/features/traces/traces_screen.dart';

void main() {
  group('TracesScreen', () {
    Widget buildUnderTest({required TracesState state}) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const TracesScreen()),
          GoRoute(
            path: '/chat',
            builder: (_, _) => const Scaffold(body: Text('chat')),
          ),
          GoRoute(
            path: '/traces/:traceId',
            builder: (_, _) => const Scaffold(body: Text('trace detail')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [
          tracesProvider.overrideWith(() => _FakeTracesNotifier(state)),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows empty state when no traces', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const TracesState()));
      await tester.pump();

      expect(find.text('No traces yet'), findsOneWidget);
    });

    testWidgets('renders trace rows for each trace in state', (tester) async {
      final state = TracesState(
        traces: [_trace('t1', 'persona-a'), _trace('t2', 'persona-b')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('persona-a'), findsOneWidget);
      expect(find.text('persona-b'), findsOneWidget);
    });

    testWidgets('formats duration in ms for short traces', (tester) async {
      final state = TracesState(traces: [_trace('t1', 'p', durationMs: 250)]);
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('250ms'), findsOneWidget);
    });

    testWidgets('formats duration in seconds for long traces', (tester) async {
      final state = TracesState(traces: [_trace('t1', 'p', durationMs: 3500)]);
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('3.5s'), findsOneWidget);
    });

    testWidgets('shows skill name when present', (tester) async {
      final state = TracesState(
        traces: [_trace('t1', 'p', skillName: 'my-skill')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('my-skill'), findsOneWidget);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final state = TracesState(error: 'Connection refused');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Connection refused'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });

    testWidgets('tapping a trace row navigates to trace detail', (
      tester,
    ) async {
      final state = TracesState(traces: [_trace('trace-abc', 'p')]);
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      await tester.tap(find.text('p'));
      await tester.pumpAndSettle();

      expect(find.text('trace detail'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

TraceSummaryResponse _trace(
  String traceId,
  String personaId, {
  int durationMs = 100,
  String status = 'ok',
  String? skillName,
}) => TraceSummaryResponse(
  (b) => b
    ..traceId = traceId
    ..personaId = personaId
    ..durationMs = durationMs
    ..status = status
    ..skillName = skillName
    ..startTime = DateTime(2024, 1, 1, 12, 0, 0),
);

// ---------------------------------------------------------------------------
// Fake notifier

class _FakeTracesNotifier extends TracesNotifier {
  _FakeTracesNotifier(this._state);
  final TracesState _state;

  @override
  Future<TracesState> build() async => _state;
}
