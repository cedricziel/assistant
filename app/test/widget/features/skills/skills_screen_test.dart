import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/features/skills/skills_provider.dart';
import 'package:assistant_app/features/skills/skills_screen.dart';

void main() {
  group('SkillsScreen', () {
    Widget buildUnderTest({
      required SkillsState skillsState,
      PersonasState? personasState,
    }) {
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const SkillsScreen()),
          GoRoute(
            path: '/chat',
            builder: (_, _) => const Scaffold(body: Text('chat')),
          ),
          GoRoute(
            path: '/skills/new',
            builder: (_, _) => const Scaffold(body: Text('new skill')),
          ),
          GoRoute(
            path: '/skills/:skillName',
            builder: (_, _) => const Scaffold(body: Text('skill detail')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [
          skillsProvider.overrideWith(() => _FakeSkillsNotifier(skillsState)),
          personasProvider.overrideWith(
            () => _FakePersonasNotifier(personasState ?? const PersonasState()),
          ),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows empty state when no skills', (tester) async {
      await tester.pumpWidget(buildUnderTest(skillsState: const SkillsState()));
      await tester.pump();

      expect(find.text('No skills configured'), findsOneWidget);
    });

    testWidgets('renders skill rows for each skill in state', (tester) async {
      final skillsState = SkillsState(
        skills: [
          _skill('bash', 'Run shell commands'),
          _skill('file-read', 'Read files'),
        ],
      );
      await tester.pumpWidget(buildUnderTest(skillsState: skillsState));
      await tester.pump();

      expect(find.text('bash'), findsOneWidget);
      expect(find.text('file-read'), findsOneWidget);
    });

    testWidgets('shows Enabled badge for enabled skill', (tester) async {
      final skillsState = SkillsState(
        skills: [_skill('bash', 'desc', enabled: true)],
      );
      await tester.pumpWidget(buildUnderTest(skillsState: skillsState));
      await tester.pump();

      expect(find.text('Enabled'), findsOneWidget);
    });

    testWidgets('shows Disabled badge for disabled skill', (tester) async {
      final skillsState = SkillsState(
        skills: [_skill('bash', 'desc', enabled: false)],
      );
      await tester.pumpWidget(buildUnderTest(skillsState: skillsState));
      await tester.pump();

      expect(find.text('Disabled'), findsOneWidget);
    });

    testWidgets('shows active persona name in app bar title', (tester) async {
      final persona = PersonaSummary(
        (b) => b
          ..id = 'default'
          ..name = 'Ada'
          ..description = '',
      );
      final personasState = PersonasState(
        personas: [persona],
        activePersona: persona,
      );
      await tester.pumpWidget(
        buildUnderTest(
          skillsState: const SkillsState(),
          personasState: personasState,
        ),
      );
      await tester.pump();

      expect(find.text('Skills — Ada'), findsOneWidget);
    });

    testWidgets('shows error view when state has error', (tester) async {
      final skillsState = SkillsState(error: 'Skills unavailable');
      await tester.pumpWidget(buildUnderTest(skillsState: skillsState));
      await tester.pump();

      expect(find.text('Skills unavailable'), findsOneWidget);
      expect(find.text('Retry'), findsOneWidget);
    });

    testWidgets('tapping skill detail icon navigates to skill detail', (
      tester,
    ) async {
      final skillsState = SkillsState(
        skills: [_skill('my-skill', 'Do things')],
      );
      await tester.pumpWidget(buildUnderTest(skillsState: skillsState));
      await tester.pump();

      await tester.tap(find.byIcon(Icons.arrow_forward_ios));
      await tester.pumpAndSettle();

      expect(find.text('skill detail'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

SkillEntryResponse _skill(
  String name,
  String description, {
  bool enabled = true,
}) => SkillEntryResponse(
  (b) => b
    ..name = name
    ..description = description
    ..enabled = enabled,
);

// ---------------------------------------------------------------------------
// Fake notifiers

class _FakeSkillsNotifier extends SkillsNotifier {
  _FakeSkillsNotifier(this._state);
  final SkillsState _state;

  @override
  Future<SkillsState> build() async => _state;
}

class _FakePersonasNotifier extends PersonasNotifier {
  _FakePersonasNotifier(this._state);
  final PersonasState _state;

  @override
  Future<PersonasState> build() async => _state;
}
