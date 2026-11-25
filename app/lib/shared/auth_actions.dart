import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../features/auth/oauth_service.dart';
import '../features/contexts/providers/context_providers.dart';
import '../features/spaces/space_provider.dart';

/// Tear down the user's session.
///
/// Revokes the session on the server first, so its tokens stop working
/// everywhere and are not merely forgotten here. Then clears the in-memory space selection BEFORE deactivating the active context
/// so the next login starts from a clean slate. The router redirect on
/// `!hasContext` then sends the user to `/login`.
///
/// Used by:
/// - The nav-shell logout button on every platform.
/// - The 401 interceptor's `onAuthExpired` callback (see
///   `connection_provider.dart`). Both paths must produce the same end state.
Future<void> performLogout(ProviderContainer container) async {
  await _revokeServerSession(container);
  container.read(spaceSelectionProvider.notifier).clear();
  await container.read(activeContextProvider.notifier).deactivate();
}

/// Asks the server to end the active context's session.
///
/// Best-effort: an unreachable server must not trap the user in a session
/// they asked to leave, so failures are ignored and the local logout goes on.
Future<void> _revokeServerSession(ProviderContainer container) async {
  try {
    final context = await container.read(activeContextProvider.future);
    final credentials = context?.oauthCredentials;
    if (context == null || credentials == null) return;
    await OAuthService(
      serverUrl: context.serverUrl,
    ).revoke(token: credentials.refreshToken, clientId: credentials.clientId);
  } catch (_) {}
}
