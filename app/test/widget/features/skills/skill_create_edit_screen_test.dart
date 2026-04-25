import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'package:assistant_app/features/skills/skill_create_edit_screen.dart';
import 'package:assistant_app/features/skills/skill_detail_provider.dart';
import 'package:assistant_app/features/skills/skills_provider.dart';

void main() {
  group('SkillCreateEditScreen – create mode', () {
    Widget buildCreateScreen({_FakeSkillsNotifier? notifier}) {
      final fake = notifier ?? _FakeSkillsNotifier();
      final router = GoRouter(
        routes: [
          GoRoute(path: '/', builder: (_, _) => const SkillCreateEditScreen()),
          GoRoute(
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('skills list')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [skillsProvider.overrideWith(() => fake)],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    // Helpers to fill in the form fields by index (name=0, desc=1, body=2).
    Future<void> fillName(WidgetTester tester, String text) =>
        tester.enterText(find.byType(TextFormField).at(0), text);
    Future<void> fillDesc(WidgetTester tester, String text) =>
        tester.enterText(find.byType(TextFormField).at(1), text);
    Future<void> fillBody(WidgetTester tester, String text) =>
        tester.enterText(find.byType(TextFormField).at(2), text);

    // Tap the AppBar "Create" button (always visible, triggers _submit).
    Future<void> tapCreate(WidgetTester tester) =>
        tester.tap(find.text('Create'));

    testWidgets('shows New Skill title and Create button', (tester) async {
      await tester.pumpWidget(buildCreateScreen());
      await tester.pump();

      expect(find.text('New Skill'), findsOneWidget);
      expect(find.text('Create'), findsOneWidget);
    });

    testWidgets('empty name shows validation error', (tester) async {
      await tester.pumpWidget(buildCreateScreen());
      await tester.pump();

      // Fill desc and body but leave name blank.
      await fillDesc(tester, 'desc');
      await fillBody(tester, 'body');
      await tapCreate(tester);
      await tester.pump();

      expect(find.text('Name is required'), findsOneWidget);
    });

    testWidgets('invalid name (uppercase) shows validation error', (
      tester,
    ) async {
      await tester.pumpWidget(buildCreateScreen());
      await tester.pump();

      await fillName(tester, 'My-Skill');
      await fillDesc(tester, 'desc');
      await fillBody(tester, 'body');
      await tapCreate(tester);
      await tester.pump();

      expect(find.textContaining('Only lowercase letters'), findsOneWidget);
    });

    testWidgets('empty description shows validation error', (tester) async {
      await tester.pumpWidget(buildCreateScreen());
      await tester.pump();

      await fillName(tester, 'my-skill');
      await fillBody(tester, 'body');
      await tapCreate(tester);
      await tester.pump();

      expect(find.text('Description is required'), findsOneWidget);
    });

    testWidgets('empty body shows validation error', (tester) async {
      await tester.pumpWidget(buildCreateScreen());
      await tester.pump();

      await fillName(tester, 'my-skill');
      await fillDesc(tester, 'desc');
      // Leave body blank.
      await tapCreate(tester);
      await tester.pump();

      expect(find.text('Body is required'), findsOneWidget);
    });

    testWidgets('valid form calls createSkill and navigates to skills list', (
      tester,
    ) async {
      final notifier = _FakeSkillsNotifier(createResult: null);
      await tester.pumpWidget(buildCreateScreen(notifier: notifier));
      await tester.pump();

      await fillName(tester, 'my-skill');
      await fillDesc(tester, 'A description');
      await fillBody(tester, '# SKILL\ncontent');
      await tapCreate(tester);
      await tester.pumpAndSettle();

      expect(notifier.createCalledWithName, equals('my-skill'));
      expect(notifier.createCalledWithDesc, equals('A description'));
      expect(find.text('skills list'), findsOneWidget);
    });

    testWidgets('server error shows inline error banner', (tester) async {
      final notifier = _FakeSkillsNotifier(createResult: 'Name already taken');
      await tester.pumpWidget(buildCreateScreen(notifier: notifier));
      await tester.pump();

      await fillName(tester, 'my-skill');
      await fillDesc(tester, 'desc');
      await fillBody(tester, 'body');
      await tapCreate(tester);
      await tester.pumpAndSettle();

      expect(find.textContaining('Name already taken'), findsOneWidget);
    });
  });

  group('SkillCreateEditScreen – edit mode', () {
    Widget buildEditScreen({required SkillDetailState detailState}) {
      final router = GoRouter(
        routes: [
          GoRoute(
            path: '/',
            builder: (_, _) =>
                const SkillCreateEditScreen(skillName: 'my-skill'),
          ),
          GoRoute(
            path: '/skills/my-skill',
            builder: (_, _) => const Scaffold(body: Text('skill detail')),
          ),
          GoRoute(
            path: '/skills',
            builder: (_, _) => const Scaffold(body: Text('skills list')),
          ),
        ],
      );
      return ProviderScope(
        overrides: [
          skillDetailProvider.overrideWith2(
            (arg) => _FakeSkillDetailNotifier(arg, detailState),
          ),
          skillsProvider.overrideWith(() => _FakeSkillsNotifier()),
        ],
        child: MaterialApp.router(routerConfig: router),
      );
    }

    testWidgets('shows Edit Skill title and Save button', (tester) async {
      await tester.pumpWidget(
        buildEditScreen(detailState: SkillDetailState(skill: _existingSkill())),
      );
      await tester.pump();

      expect(find.text('Edit Skill'), findsOneWidget);
      expect(find.text('Save'), findsOneWidget);
    });

    testWidgets('name field is read-only in edit mode', (tester) async {
      await tester.pumpWidget(
        buildEditScreen(detailState: SkillDetailState(skill: _existingSkill())),
      );
      await tester.pump();

      // The name TextFormField uses readOnly instead of enabled=false so that
      // Material 3 dark theme does not render a light disabled fill.
      final nameFields = tester
          .widgetList<TextField>(find.byType(TextField))
          .toList();

      // First field is name (in layout order: name, description, body).
      expect(nameFields.first.readOnly, isTrue);
    });
  });
}

// ---------------------------------------------------------------------------
// Fixtures

SkillDetail _existingSkill() => SkillDetail(
  (b) => b
    ..name = 'my-skill'
    ..description = 'Existing description'
    ..body = '# SKILL.md\nExisting body'
    ..source_ = 'user'
    ..isBuiltin = false,
);

// ---------------------------------------------------------------------------
// Fake notifiers

class _FakeSkillDetailNotifier extends SkillDetailNotifier {
  _FakeSkillDetailNotifier(super.skillName, this._state);
  final SkillDetailState _state;

  @override
  Future<SkillDetailState> build() async => _state;
}

class _FakeSkillsNotifier extends SkillsNotifier {
  _FakeSkillsNotifier({this.createResult});

  final String? createResult;

  String? createCalledWithName;
  String? createCalledWithDesc;

  @override
  Future<SkillsState> build() async => const SkillsState();

  @override
  Future<String?> createSkill({
    required String name,
    required String description,
    required String body,
  }) async {
    createCalledWithName = name;
    createCalledWithDesc = description;
    return createResult;
  }

  @override
  Future<String?> updateSkill({
    required String name,
    required String description,
    required String body,
  }) async {
    return null;
  }
}
