import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:fresh_dio/fresh_dio.dart';

import '../../api/api_client.dart';
import '../../api/context_token_storage.dart';
import '../../api/models/server_profile.dart';
import '../auth/oauth_service.dart';
import '../contexts/models/context_model.dart';
import '../contexts/providers/context_providers.dart';
import '../spaces/space_provider.dart';

/// Runtime connection state derived from the active [AssistantContext].
class ServerConnectionState {
  const ServerConnectionState({this.profile});

  /// `null` means no active context is selected.
  final ServerProfile? profile;

  bool get isConnected => profile != null;
}

/// Derives [ServerConnectionState] from [activeContextProvider].
///
/// No credentials are stored here — everything lives in [ContextRepository].
class ServerProfileNotifier extends AsyncNotifier<ServerConnectionState> {
  @override
  Future<ServerConnectionState> build() async {
    final activeContext = await ref.watch(activeContextProvider.future);
    if (activeContext == null) return const ServerConnectionState();
    final profile = ServerProfile(
      baseUrl: activeContext.serverUrl,
      token: activeContext.effectiveToken ?? '',
      label: activeContext.name,
    );
    return ServerConnectionState(profile: profile);
  }
}

/// Provider for the [ServerProfileNotifier].
final serverProfileProvider =
    AsyncNotifierProvider<ServerProfileNotifier, ServerConnectionState>(
      ServerProfileNotifier.new,
    );

/// Convenience provider — `true` when an active context is set.
final isConnectedProvider = Provider<bool>((ref) {
  return ref.watch(serverProfileProvider).value?.isConnected ?? false;
});

/// Provides the active [ServerProfile], or `null` if no context is active.
final activeProfileProvider = Provider<ServerProfile?>((ref) {
  return ref.watch(serverProfileProvider).value?.profile;
});

/// Provides a configured [ApiClient] for the active context, or `null` when
/// no context is selected or the context is still loading.
///
/// All feature providers should watch this in their [AsyncNotifier.build]
/// so they rebuild reactively when the connection becomes available.
final apiClientProvider = Provider<ApiClient?>((ref) {
  final ctxAsync = ref.watch(activeContextProvider);
  final ctx = ctxAsync.value;
  if (ctx == null) return null;

  // OAuth2 context with credentials → fresh_dio path (proactive refresh,
  // reactive 401 retry, single-flight, auth-status stream).
  if (ctx.authMode == AuthMode.oauth2 && ctx.oauthCredentials != null) {
    final repo = ref.watch(contextRepositoryProvider);
    final storage = ContextTokenStorage(
      contextId: ctx.id,
      repository: repo,
      initial: ctx.oauthCredentials,
    );
    final client = ApiClient.oauth2(
      baseUrl: ctx.serverUrl,
      tokenStorage: storage,
      refreshToken: (oldCreds, _) async {
        if (oldCreds == null) throw RevokeTokenException();
        try {
          final service = OAuthService(serverUrl: ctx.serverUrl);
          return await service.refresh(
            refreshToken: oldCreds.refreshToken,
            clientId: oldCreds.clientId,
          );
        } catch (_) {
          throw RevokeTokenException();
        }
      },
      onAuthExpired: () => _handleAuthExpired(ref),
    );
    ref.onDispose(client.close);
    return client;
  }

  // Legacy / API-key path: static bearer, no refresh.
  return ApiClient(
    baseUrl: ctx.serverUrl,
    token: ctx.effectiveToken ?? '',
    refreshTokens: () => _refreshAccessToken(ref),
    onAuthExpired: () => _handleAuthExpired(ref),
  );
});

/// Attempt an OAuth2 refresh for the active context. Returns the new access
/// token on success, or `null` when the active context has no refresh token,
/// is not OAuth2, or the refresh request itself fails.
Future<String?> _refreshAccessToken(Ref ref) async {
  final ctx = ref.read(activeContextProvider).value;
  if (ctx == null) return null;
  final creds = ctx.oauthCredentials;
  if (creds == null) return null;

  try {
    final service = OAuthService(serverUrl: ctx.serverUrl);
    final newCreds = await service.refresh(
      refreshToken: creds.refreshToken,
      clientId: creds.clientId,
    );
    final updated = ctx.copyWith(oauthCredentials: newCreds);
    await ref.read(contextsProvider.notifier).saveContext(updated);
    return newCreds.bearerToken;
  } catch (_) {
    return null;
  }
}

/// Tear down the session: clear the in-memory space selection then deactivate
/// the active context. The router redirect on `!hasContext` then sends the
/// user to `/login`.
Future<void> _handleAuthExpired(Ref ref) async {
  ref.read(spaceSelectionProvider.notifier).clear();
  await ref.read(activeContextProvider.notifier).deactivate();
}

/// Returns `true` when running in a web browser.
bool get isWebPlatform => kIsWeb;

/// Returns `true` when running on macOS desktop.
bool get isMacOSPlatform =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.macOS;

/// Returns `true` when running on iOS.
bool get isIOSPlatform =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.iOS;
