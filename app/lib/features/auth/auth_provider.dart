import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../contexts/models/context_model.dart';
import '../contexts/providers/context_providers.dart';
import 'browser_sign_in.dart';
import 'oauth_credentials.dart';
import 'oauth_service.dart';
import 'pkce.dart';

/// State of an in-progress OAuth2 login flow.
sealed class OAuthLoginState {
  const OAuthLoginState();
}

class OAuthLoginIdle extends OAuthLoginState {
  const OAuthLoginIdle();
}

class OAuthLoginLoading extends OAuthLoginState {
  const OAuthLoginLoading({this.message = 'Connecting...'});
  final String message;
}

class OAuthLoginSuccess extends OAuthLoginState {
  const OAuthLoginSuccess(this.context);
  final AssistantContext context;
}

class OAuthLoginError extends OAuthLoginState {
  const OAuthLoginError(this.message);
  final String message;
}

/// Builds the [OAuthService] for a server URL. Tests override it.
final oauthServiceFactoryProvider =
    Provider<OAuthService Function(String serverUrl)>(
      (ref) =>
          (serverUrl) => OAuthService(serverUrl: serverUrl),
    );

/// Manages the OAuth2 login flow for the Flutter app.
///
/// Orchestrates: server detection → client registration → login →
/// token storage → context activation.
class OAuthLoginNotifier extends Notifier<OAuthLoginState> {
  @override
  OAuthLoginState build() => const OAuthLoginIdle();

  /// Signs in on the server's own sign-in page.
  ///
  /// The server asks for the email, finds the org and shows its password
  /// form or redirects to the org's identity provider (OIDC), then redirects
  /// to the app with an authorization code. On native platforms that
  /// redirect comes straight back here; on web it lands on
  /// [browserSignInCallbackPath], where [completeBrowserSignIn] picks up.
  Future<void> startBrowserSignIn({required String serverUrl}) async {
    state = const OAuthLoginLoading(message: 'Registering client...');
    try {
      final browser = ref.read(browserSignInPlatformProvider);
      final service = ref.read(oauthServiceFactoryProvider)(serverUrl);
      final redirectUri = browser.redirectUri;
      final clientId = await service.registerClient(
        clientName: 'Assistant Flutter App',
        redirectUris: [redirectUri],
      );

      final pkce = PkceChallenge.generate();
      final pending = PendingBrowserSignIn(
        serverUrl: serverUrl,
        clientId: clientId,
        redirectUri: redirectUri,
        verifier: pkce.verifier,
        state: PkceChallenge.generate().verifier,
      );
      browser.savePending(pending);

      state = const OAuthLoginLoading(message: 'Waiting for sign-in...');
      final callback = await browser.open(
        service.buildAuthorizationUrl(
          clientId: clientId,
          redirectUri: redirectUri,
          pkce: pkce,
          state: pending.state,
        ),
      );
      if (callback != null) {
        await completeBrowserSignIn(callback.queryParameters);
      }
    } on BrowserSignInCancelled {
      ref.read(browserSignInPlatformProvider).takePending();
      state = const OAuthLoginError('Sign-in was cancelled.');
    } on OAuthException catch (e) {
      state = OAuthLoginError(e.message);
    } catch (e) {
      state = OAuthLoginError('Login failed: $e');
    }
  }

  /// Finishes a sign-in started by [startBrowserSignIn], given the query
  /// parameters the server redirected back with.
  Future<void> completeBrowserSignIn(Map<String, String> query) async {
    state = const OAuthLoginLoading(message: 'Signing in...');
    final pending = ref.read(browserSignInPlatformProvider).takePending();
    if (pending == null) {
      state = const OAuthLoginError(
        'This sign-in has expired. Please sign in again.',
      );
      return;
    }
    if (query['state'] != pending.state) {
      state = const OAuthLoginError(
        'Sign-in response did not match this browser. Please sign in again.',
      );
      return;
    }
    final error = query['error'];
    if (error != null) {
      final description = query['error_description'];
      state = OAuthLoginError(
        'Sign-in failed: ${description == null ? error : '$error ($description)'}',
      );
      return;
    }
    final code = query['code'];
    if (code == null || code.isEmpty) {
      state = const OAuthLoginError('No authorization code in server response');
      return;
    }

    try {
      final creds = await ref
          .read(oauthServiceFactoryProvider)(pending.serverUrl)
          .exchangeCode(
            code: code,
            clientId: pending.clientId,
            redirectUri: pending.redirectUri,
            pkceVerifier: pending.verifier,
          );
      await _activateOAuthContext(pending.serverUrl, creds);
    } on OAuthException catch (e) {
      state = OAuthLoginError(e.message);
    } catch (e) {
      state = OAuthLoginError('Login failed: $e');
    }
  }

  Future<void> _activateOAuthContext(
    String serverUrl,
    OAuthCredentials creds,
  ) async {
    state = const OAuthLoginLoading(message: 'Saving credentials...');
    final newCtx = AssistantContext.create(
      name: Uri.parse(serverUrl).host,
      serverUrl: serverUrl,
      oauthCredentials: creds,
      authMode: AuthMode.oauth2,
    );
    final saved = await ref
        .read(contextsProvider.notifier)
        .upsertContextByUrl(newCtx);
    await ref.read(activeContextProvider.notifier).activate(saved);
    state = OAuthLoginSuccess(saved);
  }

  /// Performs a legacy token login (for servers without OAuth2).
  Future<void> legacyLogin({required String serverUrl, String? token}) async {
    state = const OAuthLoginLoading(message: 'Connecting...');

    try {
      final serverHost = Uri.parse(serverUrl).host;
      final newCtx = AssistantContext.create(
        name: serverHost,
        serverUrl: serverUrl,
        authToken: token,
        authMode: AuthMode.legacyToken,
      );

      final saved = await ref
          .read(contextsProvider.notifier)
          .upsertContextByUrl(newCtx);
      await ref.read(activeContextProvider.notifier).activate(saved);

      state = OAuthLoginSuccess(saved);
    } catch (e) {
      state = OAuthLoginError('Connection failed: $e');
    }
  }
}

final oauthLoginProvider =
    NotifierProvider<OAuthLoginNotifier, OAuthLoginState>(
      OAuthLoginNotifier.new,
    );

/// Refreshes OAuth2 tokens for the active context if needed.
///
/// Returns the (possibly updated) context. If refresh fails, returns the
/// original context unchanged (callers should handle 401 errors).
Future<AssistantContext> refreshIfNeeded(
  WidgetRef ref,
  AssistantContext context,
) async {
  if (!context.needsTokenRefresh) return context;

  final creds = context.oauthCredentials!;
  final service = OAuthService(serverUrl: context.serverUrl);

  try {
    final newCreds = await service.refresh(
      refreshToken: creds.refreshToken,
      clientId: creds.clientId,
    );

    final updated = context.copyWith(oauthCredentials: newCreds);
    await ref.read(contextsProvider.notifier).saveContext(updated);
    return updated;
  } catch (_) {
    // Refresh failed — caller will get a 401 and can redirect to login.
    return context;
  }
}
