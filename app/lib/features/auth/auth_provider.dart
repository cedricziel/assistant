import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../contexts/models/context_model.dart';
import '../contexts/providers/context_providers.dart';
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

/// Manages the OAuth2 login flow for the Flutter app.
///
/// Orchestrates: server detection → client registration → login →
/// token storage → context activation.
class OAuthLoginNotifier extends Notifier<OAuthLoginState> {
  @override
  OAuthLoginState build() => const OAuthLoginIdle();

  /// Performs a full OAuth2 login:
  /// 1. Check server supports OAuth2
  /// 2. Register a dynamic client
  /// 3. Post credentials (email + password) to get an auth code
  /// 4. Exchange code for tokens
  /// 5. Save credentials and activate context
  Future<void> login({
    required String serverUrl,
    required String email,
    required String password,
  }) async {
    state = const OAuthLoginLoading(message: 'Checking server...');

    try {
      final service = OAuthService(serverUrl: serverUrl);

      // 1. Verify server supports OAuth2.
      final hasOAuth = await service.supportsOAuth();
      if (!hasOAuth) {
        state = const OAuthLoginError(
          'This server does not support OAuth2 authentication. '
          'Use the legacy token login instead.',
        );
        return;
      }

      // 2. Register a dynamic client.
      state = const OAuthLoginLoading(message: 'Registering client...');
      final redirectUri = _redirectUri(serverUrl);
      final clientId = await service.registerClient(
        clientName: 'Assistant Flutter App',
        redirectUris: [redirectUri],
      );

      // 3. Generate PKCE and login.
      state = const OAuthLoginLoading(message: 'Authenticating...');
      final pkce = PkceChallenge.generate();
      final creds = await service.login(
        email: email,
        password: password,
        clientId: clientId,
        redirectUri: redirectUri,
        pkce: pkce,
      );

      // 4. Create and save context.
      state = const OAuthLoginLoading(message: 'Saving credentials...');
      final serverHost = Uri.parse(serverUrl).host;
      final newCtx = AssistantContext.create(
        name: serverHost,
        serverUrl: serverUrl,
        oauthCredentials: creds,
        authMode: AuthMode.oauth2,
      );

      final saved = await ref
          .read(contextsProvider.notifier)
          .upsertContextByUrl(newCtx);
      await ref.read(activeContextProvider.notifier).activate(saved);

      state = OAuthLoginSuccess(saved);
    } on OAuthException catch (e) {
      state = OAuthLoginError(e.message);
    } catch (e) {
      state = OAuthLoginError('Login failed: $e');
    }
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

  /// The redirect URI for OAuth2 flows.
  ///
  /// On web, this is the same origin with `/oauth/callback`.
  /// On macOS, this uses a loopback address (the server accepts this).
  String _redirectUri(String serverUrl) {
    final uri = Uri.parse(serverUrl);
    return uri.replace(path: '/oauth/callback').toString();
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
