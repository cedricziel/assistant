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

/// Switches the login screen to legacy token mode by tapping the mode toggle.
Future<void> _switchToLegacyMode(WidgetTester tester) async {
  await tester.tap(find.text('Use token authentication instead'));
  await tester.pumpAndSettle();
}

/// Enters a valid server URL so form validation passes in test (non-web).
Future<void> _enterServerUrl(
  WidgetTester tester, [
  String url = 'http://localhost:8080',
]) async {
  await tester.enterText(find.widgetWithText(TextFormField, 'Server URL'), url);
}

void main() {
  group('LoginScreen', () {
    group('OAuth2 mode (default)', () {
      testWidgets('renders Sign in heading', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        expect(find.text('Sign in'), findsWidgets);
      });

      testWidgets('renders email and password fields', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        expect(find.widgetWithText(TextFormField, 'Email'), findsOneWidget);
        expect(find.widgetWithText(TextFormField, 'Password'), findsOneWidget);
      });

      testWidgets('renders Sign in button', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        expect(find.widgetWithText(FilledButton, 'Sign in'), findsOneWidget);
      });

      testWidgets('shows mode switcher to legacy token', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        expect(find.text('Use token authentication instead'), findsOneWidget);
      });

      testWidgets('validates empty email', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        // Leave email empty, enter password.
        await tester.enterText(
          find.widgetWithText(TextFormField, 'Password'),
          'secret',
        );
        await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
        await tester.pumpAndSettle();

        expect(find.text('Email is required'), findsOneWidget);
      });

      testWidgets('validates empty password', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        // Enter email, leave password empty.
        await tester.enterText(
          find.widgetWithText(TextFormField, 'Email'),
          'alice@example.com',
        );
        await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
        await tester.pumpAndSettle();

        expect(find.text('Password is required'), findsOneWidget);
      });
    });

    group('legacy token mode', () {
      testWidgets('renders token input after switching mode', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();
        await _switchToLegacyMode(tester);

        expect(
          find.widgetWithText(TextFormField, 'Token (optional)'),
          findsOneWidget,
        );
      });

      testWidgets('renders Connect button', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();
        await _switchToLegacyMode(tester);

        expect(find.widgetWithText(FilledButton, 'Connect'), findsOneWidget);
      });

      testWidgets('shows mode switcher back to OAuth2', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();
        await _switchToLegacyMode(tester);

        expect(find.text('Sign in with email and password'), findsOneWidget);
      });

      testWidgets('submitting with empty token creates context and navigates', (
        tester,
      ) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();
        await _enterServerUrl(tester);
        await _switchToLegacyMode(tester);

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
        await _enterServerUrl(tester);
        await _switchToLegacyMode(tester);

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
        await _enterServerUrl(tester);
        await _switchToLegacyMode(tester);
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
        await _enterServerUrl(tester);
        await _switchToLegacyMode(tester);
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

    group('mode switching', () {
      testWidgets('toggles between OAuth2 and legacy modes', (tester) async {
        final repo = await _makeRepo();
        await tester.pumpWidget(_buildSubject(repo));
        await tester.pumpAndSettle();

        // Starts in OAuth2 mode.
        expect(find.widgetWithText(TextFormField, 'Email'), findsOneWidget);
        expect(
          find.widgetWithText(TextFormField, 'Token (optional)'),
          findsNothing,
        );

        // Switch to legacy.
        await _switchToLegacyMode(tester);
        expect(find.widgetWithText(TextFormField, 'Email'), findsNothing);
        expect(
          find.widgetWithText(TextFormField, 'Token (optional)'),
          findsOneWidget,
        );

        // Switch back to OAuth2.
        await tester.tap(find.text('Sign in with email and password'));
        await tester.pumpAndSettle();
        expect(find.widgetWithText(TextFormField, 'Email'), findsOneWidget);
        expect(
          find.widgetWithText(TextFormField, 'Token (optional)'),
          findsNothing,
        );
      });
    });
  });
}
