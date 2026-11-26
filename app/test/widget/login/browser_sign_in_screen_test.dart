import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/auth/auth_provider.dart';
import 'package:assistant_app/features/auth/browser_sign_in.dart';
import 'package:assistant_app/features/auth/oauth_service.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';
import 'package:assistant_app/features/login/login_callback_screen.dart';
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

class _FakeBrowser implements BrowserSignInPlatform {
  _FakeBrowser({this.hostedServerUrl = 'https://assistant.example.com'});

  PendingBrowserSignIn? pending;
  Uri? navigatedTo;

  @override
  final String? hostedServerUrl;
  @override
  String get redirectUri => hostedServerUrl == null
      ? 'com.cedricziel.assistant:/oauth/callback'
      : '$hostedServerUrl/login/callback';
  @override
  void savePending(PendingBrowserSignIn value) => pending = value;
  @override
  PendingBrowserSignIn? takePending() {
    final value = pending;
    pending = null;
    return value;
  }

  @override
  Future<Uri?> open(Uri url) async {
    navigatedTo = url;
    return null;
  }
}

class _FakeOAuthService extends OAuthService {
  _FakeOAuthService(String serverUrl) : super(serverUrl: serverUrl);

  @override
  Future<String> registerClient({
    required String clientName,
    required List<String> redirectUris,
  }) async => 'client_1';
}

Future<Widget> _buildSubject(_FakeBrowser browser, String location) async {
  SharedPreferences.setMockInitialValues({});
  final prefs = await SharedPreferences.getInstance();
  final repo = ContextRepository(
    prefs: prefs,
    secureStorage: _FakeSecureStorage(),
  );
  return ProviderScope(
    overrides: [
      contextRepositoryProvider.overrideWithValue(repo),
      browserSignInPlatformProvider.overrideWithValue(browser),
      oauthServiceFactoryProvider.overrideWithValue(_FakeOAuthService.new),
    ],
    child: MaterialApp.router(
      routerConfig: GoRouter(
        initialLocation: location,
        routes: [
          GoRoute(path: '/login', builder: (ctx, s) => const LoginScreen()),
          GoRoute(
            path: browserSignInCallbackPath,
            builder: (ctx, s) =>
                LoginCallbackScreen(query: s.uri.queryParameters),
          ),
          GoRoute(path: '/chat', builder: (ctx, s) => const Text('chat')),
        ],
      ),
    ),
  );
}

void main() {
  group('LoginScreen with browser sign-in', () {
    testWidgets('has no in-app password form', (tester) async {
      await tester.pumpWidget(await _buildSubject(_FakeBrowser(), '/login'));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(TextFormField, 'Email'), findsNothing);
      expect(find.widgetWithText(TextFormField, 'Password'), findsNothing);
    });

    testWidgets('Sign in sends the browser to the server sign-in page', (
      tester,
    ) async {
      final browser = _FakeBrowser();
      await tester.pumpWidget(await _buildSubject(browser, '/login'));
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
      // The button keeps spinning while the browser leaves the page.
      await tester.pump();

      expect(browser.navigatedTo?.path, '/oauth/authorize');
      expect(
        browser.navigatedTo?.queryParameters['redirect_uri'],
        'https://assistant.example.com/login/callback',
      );
    });
    testWidgets('native: asks for the server, then opens its sign-in page', (
      tester,
    ) async {
      final browser = _FakeBrowser(hostedServerUrl: null);
      await tester.pumpWidget(await _buildSubject(browser, '/login'));
      await tester.pumpAndSettle();

      expect(find.widgetWithText(TextFormField, 'Password'), findsNothing);
      await tester.enterText(
        find.widgetWithText(TextFormField, 'Server URL'),
        'https://my-server.example.com',
      );
      await tester.tap(find.widgetWithText(FilledButton, 'Sign in'));
      await tester.pump();

      expect(browser.navigatedTo?.host, 'my-server.example.com');
      expect(browser.navigatedTo?.path, '/oauth/authorize');
      expect(
        browser.navigatedTo?.queryParameters['redirect_uri'],
        'com.cedricziel.assistant:/oauth/callback',
      );
    });
  });

  group('LoginCallbackScreen', () {
    testWidgets('shows the error and a way back when sign-in fails', (
      tester,
    ) async {
      // No pending sign-in: the callback cannot be completed.
      await tester.pumpWidget(
        await _buildSubject(
          _FakeBrowser(),
          '/login/callback?code=abc&state=xyz',
        ),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('expired'), findsOneWidget);

      await tester.tap(find.text('Back to sign in'));
      await tester.pumpAndSettle();
      expect(find.byType(LoginScreen), findsOneWidget);
    });
  });
}
