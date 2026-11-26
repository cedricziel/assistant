import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/persona_detail_screen.dart';
import 'package:assistant_app/features/personas/persona_starters_editor_screen.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../../support/fake_personas_api.dart';

Widget _buildUnderTest(FakePersonasApi api) {
  final router = GoRouter(
    initialLocation: '/personas/p1',
    routes: [
      GoRoute(
        path: '/personas/:id',
        builder: (_, state) =>
            PersonaDetailScreen(personaId: state.pathParameters['id']!),
        routes: [
          GoRoute(
            path: 'starters',
            builder: (_, state) => PersonaStartersEditorScreen(
              personaId: state.pathParameters['id']!,
            ),
          ),
        ],
      ),
      GoRoute(path: '/personas', builder: (_, _) => const Text('personas')),
    ],
  );
  return ProviderScope(
    overrides: [
      apiClientProvider.overrideWithValue(FakePersonasApiClient(api)),
    ],
    child: MaterialApp.router(routerConfig: router),
  );
}

Future<void> _openEditor(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key('starters_edit')));
  await tester.pumpAndSettle();
}

void main() {
  group('Starter prompts section on the detail screen', () {
    testWidgets('shows the section with a count even without starters', (
      tester,
    ) async {
      await tester.pumpWidget(_buildUnderTest(FakePersonasApi()));
      await tester.pumpAndSettle();

      expect(find.text('Starter prompts'), findsOneWidget);
      expect(find.text('0 of 4'), findsOneWidget);
      expect(find.byKey(const Key('starters_edit')), findsOneWidget);
    });

    testWidgets('lists the prompts and opens the editor', (tester) async {
      await tester.pumpWidget(
        _buildUnderTest(FakePersonasApi(starters: ['Plan my week', 'Triage'])),
      );
      await tester.pumpAndSettle();

      expect(find.text('2 of 4'), findsOneWidget);
      expect(find.text('Plan my week'), findsOneWidget);

      await _openEditor(tester);
      expect(find.byType(PersonaStartersEditorScreen), findsOneWidget);
      expect(find.byKey(const Key('starter_field_0')), findsOneWidget);
    });

    testWidgets('is read-only without edit permission', (tester) async {
      await tester.pumpWidget(
        _buildUnderTest(
          FakePersonasApi(starters: ['Plan my week'], canEdit: false),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('Plan my week'), findsOneWidget);
      expect(find.byKey(const Key('starters_edit')), findsNothing);
    });
  });

  group('PersonaStartersEditorScreen', () {
    testWidgets('adds a prompt, saves, and the detail shows it', (
      tester,
    ) async {
      final api = FakePersonasApi();
      await tester.pumpWidget(_buildUnderTest(api));
      await tester.pumpAndSettle();
      await _openEditor(tester);

      await tester.tap(find.byKey(const Key('starter_add')));
      await tester.pump();
      await tester.enterText(
        find.byKey(const Key('starter_field_0')),
        'Plan my week',
      );
      await tester.pump();
      await tester.tap(find.byKey(const Key('starters_save')));
      await tester.pumpAndSettle();

      expect(api.updates.single, ['Plan my week']);
      expect(find.byType(PersonaStartersEditorScreen), findsNothing);
      expect(find.text('Plan my week'), findsOneWidget);
      expect(find.text('1 of 4'), findsOneWidget);
    });

    testWidgets('edits, reorders and removes prompts', (tester) async {
      final api = FakePersonasApi(starters: ['A', 'B', 'C']);
      await tester.pumpWidget(_buildUnderTest(api));
      await tester.pumpAndSettle();
      await _openEditor(tester);

      await tester.enterText(find.byKey(const Key('starter_field_0')), 'A2');
      await tester.tap(find.byKey(const Key('starter_down_0')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('starter_remove_2')));
      await tester.pump();
      await tester.tap(find.byKey(const Key('starters_save')));
      await tester.pumpAndSettle();

      expect(api.updates.single, ['B', 'A2']);
    });

    testWidgets('disables add at 4 prompts', (tester) async {
      await tester.pumpWidget(
        _buildUnderTest(FakePersonasApi(starters: ['A', 'B', 'C', 'D'])),
      );
      await tester.pumpAndSettle();
      await _openEditor(tester);

      final add = tester.widget<ButtonStyleButton>(
        find.byKey(const Key('starter_add')),
      );
      expect(add.onPressed, isNull);
    });

    testWidgets('blocks saving blank or over-long prompts', (tester) async {
      final api = FakePersonasApi(starters: ['A']);
      await tester.pumpWidget(_buildUnderTest(api));
      await tester.pumpAndSettle();
      await _openEditor(tester);

      await tester.enterText(find.byKey(const Key('starter_field_0')), '   ');
      await tester.pump();
      expect(find.text('Starter prompts cannot be blank.'), findsOneWidget);
      expect(
        tester
            .widget<ButtonStyleButton>(find.byKey(const Key('starters_save')))
            .onPressed,
        isNull,
      );

      await tester.enterText(
        find.byKey(const Key('starter_field_0')),
        'x' * 201,
      );
      await tester.pump();
      expect(find.textContaining('limited to 200'), findsOneWidget);
      expect(api.updates, isEmpty);
    });

    testWidgets('shows an error and stays when the save fails', (tester) async {
      final api = FakePersonasApi(starters: ['A'])
        ..updateError = Exception('server said no');
      await tester.pumpWidget(_buildUnderTest(api));
      await tester.pumpAndSettle();
      await _openEditor(tester);

      await tester.enterText(find.byKey(const Key('starter_field_0')), 'B');
      await tester.pump();
      await tester.tap(find.byKey(const Key('starters_save')));
      await tester.pumpAndSettle();

      expect(find.byType(PersonaStartersEditorScreen), findsOneWidget);
      expect(find.textContaining('server said no'), findsOneWidget);
    });
  });
}
