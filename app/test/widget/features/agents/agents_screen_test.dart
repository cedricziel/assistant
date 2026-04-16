import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/agents/agents_provider.dart';
import 'package:assistant_app/features/agents/agents_screen.dart';

void main() {
  group('AgentsScreen', () {
    Widget buildUnderTest({required AgentsState state}) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const AgentsScreen()),
          GoRoute(
            path: '/agents/:agentId',
            builder: (_, _) => const Scaffold(body: Text('agent detail')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [
          agentsProvider.overrideWith(() => _FakeAgentsNotifier(state)),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows empty state when no agents', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const AgentsState()));
      await tester.pump();

      expect(find.text('No agents registered'), findsOneWidget);
    });

    testWidgets('renders agent rows for each agent in state', (tester) async {
      final state = AgentsState(
        agents: [_agent('a1', 'Alpha Bot'), _agent('a2', 'Beta Bot')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Alpha Bot'), findsOneWidget);
      expect(find.text('Beta Bot'), findsOneWidget);
    });

    testWidgets('shows skill count in subtitle', (tester) async {
      final state = AgentsState(
        agents: [_agent('a1', 'My Agent', skillCount: 3)],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.textContaining('3 skills'), findsOneWidget);
    });

    testWidgets('shows singular skill label when skillCount is 1', (
      tester,
    ) async {
      final state = AgentsState(
        agents: [_agent('a1', 'Solo Agent', skillCount: 1)],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.textContaining('1 skill'), findsOneWidget);
      expect(find.textContaining('1 skills'), findsNothing);
    });

    testWidgets('shows Default badge for default agent', (tester) async {
      final state = AgentsState(
        agents: [_agent('a1', 'Default Agent', isDefault: true)],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Default'), findsOneWidget);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final state = AgentsState(error: 'Connection refused');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Connection refused'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });

    testWidgets('tapping an agent row navigates to agent detail', (
      tester,
    ) async {
      final state = AgentsState(agents: [_agent('agent-xyz', 'Nav Agent')]);
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      await tester.tap(find.text('Nav Agent'));
      await tester.pumpAndSettle();

      expect(find.text('agent detail'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

AgentSummary _agent(
  String id,
  String name, {
  int skillCount = 2,
  bool isDefault = false,
  String version = '1.0',
}) => AgentSummary(
  (b) => b
    ..id = id
    ..name = name
    ..description = 'Test agent $name'
    ..skillCount = skillCount
    ..interfaceCount = 0
    ..isDefault = isDefault
    ..version = version,
);

// ---------------------------------------------------------------------------
// Fake notifier

class _FakeAgentsNotifier extends AgentsNotifier {
  _FakeAgentsNotifier(this._state);
  final AgentsState _state;

  @override
  Future<AgentsState> build() async => _state;
}
