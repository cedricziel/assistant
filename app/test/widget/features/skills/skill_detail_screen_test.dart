import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/skills/skill_detail_provider.dart';
import 'package:assistant_app/features/skills/skill_detail_screen.dart';
import 'package:assistant_app/features/skills/skills_provider.dart';

void main() {
  group('SkillDetailScreen', () {
    // Build a minimal router so context.go() calls don't throw.
    Widget buildUnderTest({
      required SkillDetailState detailState,
      _FakeSkillsNotifier? skillsNotifier,
    }) {
      final fake = skillsNotifier ?? _FakeSkillsNotifier();
      final router = GoRouter(
        routes: [
          GoRoute(
            path: '/',
            builder: (_, _) =>
                const SkillDetailScreen(skillName: 'my-skill'),
          ),
          GoRoute(
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('skills list')),
          ),
          GoRoute(
            path: '/skills/:name/edit',
            builder: (_, _) =>
                const Scaffold(body: Text('edit screen')),
          ),
        ],
      );

      return ProviderScope(
        overrides: [
          skillDetailProvider.overrideWith2(
            (arg) => _FakeSkillDetailNotifier(arg, detailState),
          ),
          skillsProvider.overrideWith(() => fake),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('edit and delete icons shown for user-created skill',
        (tester) async {
      await tester.pumpWidget(
        buildUnderTest(detailState: SkillDetailState(skill: _userSkill())),
      );
      await tester.pump();

      expect(find.byIcon(Icons.edit_outlined), findsOneWidget);
      expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    });

    testWidgets('edit and delete icons hidden for builtin skill',
        (tester) async {
      await tester.pumpWidget(
        buildUnderTest(detailState: SkillDetailState(skill: _builtinSkill())),
      );
      await tester.pump();

      expect(find.byIcon(Icons.edit_outlined), findsNothing);
      expect(find.byIcon(Icons.delete_outline), findsNothing);
    });

    testWidgets('tapping delete shows confirmation dialog', (tester) async {
      await tester.pumpWidget(
        buildUnderTest(detailState: SkillDetailState(skill: _userSkill())),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      expect(find.byType(AlertDialog), findsOneWidget);
      expect(find.text('Delete skill?'), findsOneWidget);
    });

    testWidgets('canceling delete dialog does not call deleteSkill',
        (tester) async {
      final notifier = _FakeSkillsNotifier();
      await tester.pumpWidget(
        buildUnderTest(
          detailState: SkillDetailState(skill: _userSkill()),
          skillsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      expect(notifier.deleteCalledWith, isNull);
      expect(find.byType(AlertDialog), findsNothing);
    });

    testWidgets('confirming delete calls deleteSkill with skill name',
        (tester) async {
      // Fake returns an error so navigation doesn't happen — lets us assert
      // the provider was called without needing a full navigation stack.
      final notifier = _FakeSkillsNotifier(deleteResult: 'simulated error');
      await tester.pumpWidget(
        buildUnderTest(
          detailState: SkillDetailState(skill: _userSkill()),
          skillsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();

      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(notifier.deleteCalledWith, equals('my-skill'));
    });

    testWidgets('delete error shows snackbar', (tester) async {
      final notifier =
          _FakeSkillsNotifier(deleteResult: 'Something went wrong');
      await tester.pumpWidget(
        buildUnderTest(
          detailState: SkillDetailState(skill: _userSkill()),
          skillsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.textContaining('Something went wrong'), findsOneWidget);
    });

    testWidgets('successful delete navigates to skills list', (tester) async {
      final notifier = _FakeSkillsNotifier(deleteResult: null);
      await tester.pumpWidget(
        buildUnderTest(
          detailState: SkillDetailState(skill: _userSkill()),
          skillsNotifier: notifier,
        ),
      );
      await tester.pump();

      await tester.tap(find.byIcon(Icons.delete_outline));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete'));
      await tester.pumpAndSettle();

      expect(find.text('skills list'), findsOneWidget);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

SkillDetail _userSkill() => SkillDetail((b) => b
  ..name = 'my-skill'
  ..description = 'A user skill'
  ..body = '# SKILL.md\nDoes things'
  ..source_ = 'user'
  ..isBuiltin = false);

SkillDetail _builtinSkill() => SkillDetail((b) => b
  ..name = 'builtin-skill'
  ..description = 'A built-in skill'
  ..body = '# SKILL.md\nBuilt in'
  ..source_ = 'builtin'
  ..isBuiltin = true);

// ---------------------------------------------------------------------------
// Fake notifiers

class _FakeSkillDetailNotifier extends SkillDetailNotifier {
  _FakeSkillDetailNotifier(super.skillName, this._state);

  final SkillDetailState _state;

  @override
  Future<SkillDetailState> build() async => _state;
}

class _FakeSkillsNotifier extends SkillsNotifier {
  _FakeSkillsNotifier({this.deleteResult});

  /// The value returned by deleteSkill. null = success, non-null = error.
  final String? deleteResult;

  /// The argument passed to deleteSkill, or null if never called.
  String? deleteCalledWith;

  @override
  Future<SkillsState> build() async => const SkillsState();

  @override
  Future<String?> deleteSkill(String name) async {
    deleteCalledWith = name;
    return deleteResult;
  }
}
