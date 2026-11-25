import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:assistant_app/features/auth/auth_provider.dart';
import 'package:assistant_app/features/auth/browser_sign_in.dart';
import 'package:assistant_app/features/auth/oauth_credentials.dart';
import 'package:assistant_app/features/auth/oauth_service.dart';
import 'package:assistant_app/features/auth/pkce.dart';
import 'package:assistant_app/features/contexts/data/context_repository.dart';
import 'package:assistant_app/features/contexts/models/context_model.dart';
import 'package:assistant_app/features/contexts/providers/context_providers.dart';

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
  _FakeBrowser({
    this.redirectUri = 'https://assistant.example.com/login/callback',
    this.respond,
  });

  PendingBrowserSignIn? pending;
  Uri? navigatedTo;

  /// Builds the callback a native sign-in sheet returns; null models the web,
  /// where the page leaves and nothing comes back.
  final Uri? Function(Uri authorizeUrl)? respond;

  @override
  final String redirectUri;

  @override
  String? get hostedServerUrl => null;

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
    return respond?.call(url);
  }
}

class _FakeOAuthService extends OAuthService {
  _FakeOAuthService(String serverUrl) : super(serverUrl: serverUrl);

  List<String>? registeredRedirectUris;
  Map<String, String>? exchanged;

  @override
  Future<String> registerClient({
    required String clientName,
    required List<String> redirectUris,
  }) async {
    registeredRedirectUris = redirectUris;
    return 'client_1';
  }

  @override
  Future<OAuthCredentials> exchangeCode({
    required String code,
    required String clientId,
    required String redirectUri,
    required String pkceVerifier,
  }) async {
    exchanged = {
      'code': code,
      'clientId': clientId,
      'redirectUri': redirectUri,
      'verifier': pkceVerifier,
    };
    return OAuthCredentials(
      accessToken: 'access',
      refreshToken: 'refresh',
      expiresAt: DateTime.now().add(const Duration(hours: 1)),
      clientId: clientId,
    );
  }
}

const _server = 'https://assistant.example.com';

Future<(ProviderContainer, _FakeBrowser, _FakeOAuthService, ContextRepository)>
_setUp({_FakeBrowser? browser}) async {
  SharedPreferences.setMockInitialValues({});
  final prefs = await SharedPreferences.getInstance();
  final repo = ContextRepository(
    prefs: prefs,
    secureStorage: _FakeSecureStorage(),
  );
  browser ??= _FakeBrowser();
  final service = _FakeOAuthService(_server);
  final container = ProviderContainer(
    overrides: [
      contextRepositoryProvider.overrideWithValue(repo),
      browserSignInPlatformProvider.overrideWithValue(browser),
      oauthServiceFactoryProvider.overrideWithValue((_) => service),
    ],
  );
  addTearDown(container.dispose);
  return (container, browser, service, repo);
}

void main() {
  group('startBrowserSignIn', () {
    test('registers the app callback and sends the browser to the server '
        'sign-in page', () async {
      final (container, browser, service, _) = await _setUp();

      await container
          .read(oauthLoginProvider.notifier)
          .startBrowserSignIn(serverUrl: _server);

      expect(service.registeredRedirectUris, [
        '$_server/login/callback',
      ], reason: 'the server must send the code back to the app route');
      final pending = browser.pending!;
      final url = browser.navigatedTo!;
      expect(url.path, '/oauth/authorize');
      expect(url.queryParameters['client_id'], 'client_1');
      expect(url.queryParameters['redirect_uri'], '$_server/login/callback');
      expect(url.queryParameters['state'], pending.state);
      expect(
        url.queryParameters['code_challenge'],
        PkceChallenge.deriveChallenge(pending.verifier),
        reason: 'the stored verifier must match the challenge sent',
      );
      expect(pending.state, isNotEmpty);
    });
    test('completes in place when the platform returns the callback '
        '(native sign-in sheet)', () async {
      const redirect = 'com.cedricziel.assistant:/oauth/callback';
      final (container, _, service, repo) = await _setUp(
        browser: _FakeBrowser(
          redirectUri: redirect,
          respond: (url) => Uri.parse(redirect).replace(
            queryParameters: {
              'code': 'native_code',
              'state': url.queryParameters['state'],
            },
          ),
        ),
      );

      await container
          .read(oauthLoginProvider.notifier)
          .startBrowserSignIn(serverUrl: _server);

      final state = container.read(oauthLoginProvider);
      expect(state, isA<OAuthLoginSuccess>());
      expect(service.registeredRedirectUris, [redirect]);
      expect(service.exchanged?['code'], 'native_code');
      expect(service.exchanged?['redirectUri'], redirect);
      expect(
        repo.getActiveContextId(),
        (state as OAuthLoginSuccess).context.id,
      );
    });

    test('reports a cancelled native sign-in as an error', () async {
      final (container, _, service, _) = await _setUp(
        browser: _FakeBrowser(
          redirectUri: 'com.cedricziel.assistant:/oauth/callback',
          respond: (_) => throw const BrowserSignInCancelled(),
        ),
      );

      await container
          .read(oauthLoginProvider.notifier)
          .startBrowserSignIn(serverUrl: _server);

      final state = container.read(oauthLoginProvider);
      expect(state, isA<OAuthLoginError>());
      expect((state as OAuthLoginError).message, contains('cancelled'));
      expect(service.exchanged, isNull);
    });
  });

  group('completeBrowserSignIn', () {
    Future<void> start(ProviderContainer container) => container
        .read(oauthLoginProvider.notifier)
        .startBrowserSignIn(serverUrl: _server);

    test('exchanges the code and activates a new context', () async {
      final (container, browser, service, repo) = await _setUp();
      await start(container);
      final pending = browser.pending!;

      await container.read(oauthLoginProvider.notifier).completeBrowserSignIn({
        'code': 'the_code',
        'state': pending.state,
      });

      final state = container.read(oauthLoginProvider);
      expect(state, isA<OAuthLoginSuccess>());
      final ctx = (state as OAuthLoginSuccess).context;
      expect(ctx.serverUrl, _server);
      expect(ctx.authMode, AuthMode.oauth2);
      expect(service.exchanged, {
        'code': 'the_code',
        'clientId': 'client_1',
        'redirectUri': '$_server/login/callback',
        'verifier': pending.verifier,
      });
      expect(repo.getActiveContextId(), ctx.id);
      expect(browser.pending, isNull, reason: 'pending sign-in is consumed');
    });

    test('rejects a state that does not match', () async {
      final (container, _, service, _) = await _setUp();
      await start(container);

      await container.read(oauthLoginProvider.notifier).completeBrowserSignIn({
        'code': 'the_code',
        'state': 'forged',
      });

      expect(container.read(oauthLoginProvider), isA<OAuthLoginError>());
      expect(service.exchanged, isNull, reason: 'no code exchange on mismatch');
    });

    test('fails when no sign-in was started in this browser', () async {
      final (container, _, service, _) = await _setUp();

      await container.read(oauthLoginProvider.notifier).completeBrowserSignIn({
        'code': 'the_code',
        'state': 'any',
      });

      expect(container.read(oauthLoginProvider), isA<OAuthLoginError>());
      expect(service.exchanged, isNull);
    });

    test('surfaces an error returned by the server', () async {
      final (container, browser, _, _) = await _setUp();
      await start(container);

      await container.read(oauthLoginProvider.notifier).completeBrowserSignIn({
        'error': 'access_denied',
        'state': browser.pending!.state,
      });

      final state = container.read(oauthLoginProvider);
      expect(state, isA<OAuthLoginError>());
      expect((state as OAuthLoginError).message, contains('access_denied'));
    });
  });
}
