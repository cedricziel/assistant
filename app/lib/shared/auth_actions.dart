import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../features/contexts/providers/context_providers.dart';
import '../features/spaces/space_provider.dart';

/// Tear down the user's session.
///
/// Clears the in-memory space selection BEFORE deactivating the active context
/// so the next login starts from a clean slate. The router redirect on
/// `!hasContext` then sends the user to `/login`.
///
/// Used by:
/// - The nav-shell logout button on every platform.
/// - The 401 interceptor's `onAuthExpired` callback (see
///   `connection_provider.dart`). Both paths must produce the same end state.
Future<void> performLogout(ProviderContainer container) async {
  container.read(spaceSelectionProvider.notifier).clear();
  await container.read(activeContextProvider.notifier).deactivate();
}
