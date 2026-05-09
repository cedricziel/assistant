import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../features/contexts/providers/context_providers.dart';
import '../features/spaces/space_provider.dart';

/// Tear down the user's session on the web.
///
/// Clears the in-memory space selection BEFORE deactivating the active context
/// so the next login starts from a clean slate. The router redirect on
/// `!hasContext` then sends the user to `/login`.
///
/// The 401 interceptor wires the same shape into its `onAuthExpired` callback
/// (see `connection_provider.dart`). Keep them consistent.
Future<void> performWebLogout(ProviderContainer container) async {
  container.read(spaceSelectionProvider.notifier).clear();
  await container.read(activeContextProvider.notifier).deactivate();
}
