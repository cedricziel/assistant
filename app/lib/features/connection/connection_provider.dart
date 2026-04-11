import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../api/models/server_profile.dart';
import '../contexts/providers/context_providers.dart';

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
      token: activeContext.authToken ?? '',
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

/// Returns `true` when running in a web browser.
bool get isWebPlatform => kIsWeb;

/// Returns `true` when running on macOS desktop.
bool get isMacOSPlatform =>
    !kIsWeb && defaultTargetPlatform == TargetPlatform.macOS;
