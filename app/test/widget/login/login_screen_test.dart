import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/login/login_screen.dart';

class _FakeSecureStorage implements SecureKeyValueStorage {
  final Map<String, String> _data = {};
  @override
  Future<String?> read({required String key}) async => _data[key];
  @override
  Future<void> write({required String key, required String value}) async =>
      _data[key] = value;
  @override
  Future<void> delete({required String key}) async => _data.remove(key);
}

Future<ContextRepository> _makeRepo() async {
  SharedPreferences.setMockInitialValues({});
  final p = await SharedPreferences.getInstance();
  return ContextRepository(prefs: p, secureStorage: _FakeSecureStorage());
}

Widget _buildSubject(ContextRepository repo) {
  return ProviderScope(
    overrides: [contextRepositoryProvider.overrideWithValue(repo)],
    child: MaterialApp.router(
      routerConfig: GoRouter(
        initialLocation: '/login',
        routes: [
          GoRoute(path: '/login', builder: (ctx, s) => const LoginScreen()),
          GoRoute(path: '/chat', builder: (ctx, s) => const Scaffold()),
        ],
      ),
    ),
  );
}

void main() {
  group('LoginScreen', () {
    testWidgets('renders server URL as read-only text', (tester) async {
      final repo = await _makeRepo();
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();

      // The InputDecorator shows a 'Server' label.
      expect(find.text('Server'), findsOneWidget);
    });

    testWidgets('renders token input field', (tester) async {
      final repo = await _makeRepo();
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();

      expect(
        find.widgetWithText(TextFormField, 'Token (optional)'),
        findsOneWidget,
      );
    });

    testWidgets('renders Connect button', (tester) async {
      final repo = await _makeRepo();
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(FilledButton, 'Connect'), findsOneWidget);
    });

    testWidgets('submitting with empty token creates context and navigates', (
      tester,
    ) async {
      final repo = await _makeRepo();
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(FilledButton, 'Connect'));
      await tester.pumpAndSettle();

      // Should navigate to /chat after successful login.
      expect(find.byType(Scaffold), findsOneWidget);

      // A context should have been created.
      final contexts = await repo.loadContexts();
      expect(contexts, hasLength(1));
    });

    testWidgets('submitting with a token persists the token', (tester) async {
      final repo = await _makeRepo();
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();

      await tester.enterText(
        find.widgetWithText(TextFormField, 'Token (optional)'),
        'my-secret-token',
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Connect'));
      await tester.pumpAndSettle();

      final contexts = await repo.loadContexts();
      expect(contexts.single.authToken, 'my-secret-token');
    });

    testWidgets('submitting twice for same URL updates existing context', (
      tester,
    ) async {
      final repo = await _makeRepo();

      // First login.
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.widgetWithText(TextFormField, 'Token (optional)'),
        'token-v1',
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Connect'));
      await tester.pumpAndSettle();

      expect((await repo.loadContexts()).length, 1);

      // Navigate back to /login manually via router and submit again.
      await tester.pumpWidget(_buildSubject(repo));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.widgetWithText(TextFormField, 'Token (optional)'),
        'token-v2',
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Connect'));
      await tester.pumpAndSettle();

      // Still only 1 context — it was updated, not duplicated.
      final contexts = await repo.loadContexts();
      expect(contexts.length, 1);
      expect(contexts.single.authToken, 'token-v2');
    });
  });
}
