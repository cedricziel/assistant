import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/features/personas/personas_screen.dart';

void main() {
  group('PersonasScreen', () {
    Widget buildUnderTest({required PersonasState state}) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const PersonasScreen()),
          GoRoute(
            path: '/personas/new',
            builder: (_, _) => const Scaffold(body: Text('create persona')),
          ),
          GoRoute(
            path: '/personas/:personaId',
            builder: (_, _) => const Scaffold(body: Text('persona detail')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [
          personasProvider.overrideWith(() => _FakePersonasNotifier(state)),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows empty state when no personas', (tester) async {
      await tester.pumpWidget(buildUnderTest(state: const PersonasState()));
      await tester.pump();

      expect(find.text('No personas found'), findsOneWidget);
    });

    testWidgets('renders persona rows for each persona in state', (
      tester,
    ) async {
      final state = PersonasState(
        personas: [_persona('p1', 'Alice'), _persona('p2', 'Bob')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Bob'), findsOneWidget);
    });

    testWidgets('shows persona id as subtitle', (tester) async {
      final state = PersonasState(
        personas: [_persona('persona-abc', 'Charlie')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('persona-abc'), findsOneWidget);
    });

    testWidgets('shows Default badge for default persona', (tester) async {
      final state = PersonasState(
        personas: [_persona('default', 'Default Persona')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Default'), findsOneWidget);
    });

    testWidgets('filters personas by search query', (tester) async {
      final state = PersonasState(
        personas: [_persona('p1', 'Alice'), _persona('p2', 'Bob')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      await tester.enterText(find.byType(TextField), 'ali');
      await tester.pump();

      expect(find.text('Alice'), findsOneWidget);
      expect(find.text('Bob'), findsNothing);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final state = PersonasState(error: 'Connection refused');
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      expect(find.text('Connection refused'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });

    testWidgets('tapping a persona row navigates to persona detail', (
      tester,
    ) async {
      final state = PersonasState(
        personas: [_persona('persona-xyz', 'Nav Persona')],
      );
      await tester.pumpWidget(buildUnderTest(state: state));
      await tester.pump();

      await tester.tap(find.text('Nav Persona'));
      await tester.pumpAndSettle();

      expect(find.text('persona detail'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

PersonaSummary _persona(String id, String name) => PersonaSummary(
  (b) => b
    ..id = id
    ..name = name
    ..description = 'Test persona $name',
);

// ---------------------------------------------------------------------------
// Fake notifier

class _FakePersonasNotifier extends PersonasNotifier {
  _FakePersonasNotifier(this._state);
  final PersonasState _state;

  @override
  Future<PersonasState> build() async => _state;
}
