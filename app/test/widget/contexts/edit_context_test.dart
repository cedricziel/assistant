import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/contexts/screens/edit_context_screen.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  @override
  Future<String?> read({required String key}) async => null;
  @override
  Future<void> write({required String key, required String value}) async {}
  @override
  Future<void> delete({required String key}) async {}
}

Future<ContextRepository> makeRepo() async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

AssistantContext makeCtx() => AssistantContext(
  id: 'ctx-1',
  name: 'Work',
  serverUrl: 'https://work.example.com',
  authToken: 'my-token',
  createdAt: DateTime.utc(2024, 1, 1),
);

Widget buildEdit(ContextRepository repo, AssistantContext ctx) {
  return ProviderScope(
    overrides: [contextRepositoryProvider.overrideWithValue(repo)],
    child: MaterialApp(home: EditContextScreen(existingContext: ctx)),
  );
}

void main() {
  group('EditContextScreen', () {
    testWidgets('pre-fills fields from existing context', (tester) async {
      final repo = await makeRepo();
      final ctx = makeCtx();
      await repo.saveContext(ctx);

      await tester.pumpWidget(buildEdit(repo, ctx));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(TextFormField, 'Work'), findsWidgets);
      expect(
        find.widgetWithText(TextFormField, 'https://work.example.com'),
        findsWidgets,
      );
    });

    testWidgets('validates empty name on save', (tester) async {
      final repo = await makeRepo();
      final ctx = makeCtx();
      await repo.saveContext(ctx);

      await tester.pumpWidget(buildEdit(repo, ctx));
      await tester.pumpAndSettle();

      // Clear the name field.
      final nameField = find.widgetWithText(TextFormField, 'Work').first;
      await tester.enterText(nameField, '');

      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      expect(find.text('Name is required'), findsOneWidget);
    });

    testWidgets('saves updated name to repository', (tester) async {
      final repo = await makeRepo();
      final ctx = makeCtx();
      await repo.saveContext(ctx);

      await tester.pumpWidget(buildEdit(repo, ctx));
      await tester.pumpAndSettle();

      final nameField = find.widgetWithText(TextFormField, 'Work').first;
      await tester.enterText(nameField, 'Work Updated');

      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();

      final saved = await repo.loadContexts();
      expect(saved.single.name, 'Work Updated');
    });
  });
}
