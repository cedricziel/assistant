import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../connection/connection_provider.dart';
import '../contexts/models/context_model.dart';
import '../contexts/providers/context_providers.dart';
import 'embedded_server_service.dart';

export 'embedded_server_service.dart'
    show
        EmbeddedServerState,
        EmbeddedServerStarting,
        EmbeddedServerReady,
        EmbeddedServerError,
        EmbeddedServerStopped;

/// Manages the embedded-server lifecycle as a Riverpod [AsyncNotifier].
///
/// On macOS, if the bundled binary is available and no remote server profile
/// is already saved, this notifier automatically starts the embedded server
/// and calls [ServerProfileNotifier.connect] when it becomes ready — causing
/// the router to redirect to `/chat` without any manual setup.
///
/// **Note:** iOS is intentionally not supported for embedded server. iOS
/// apps cannot bundle and execute arbitrary binaries due to platform
/// restrictions, so this notifier always returns [EmbeddedServerStopped]
/// on iOS devices.
class EmbeddedServerNotifier extends AsyncNotifier<EmbeddedServerState>
    with WidgetsBindingObserver {
  final _service = EmbeddedServerService();

  @override
  Future<EmbeddedServerState> build() async {
    // Register for app lifecycle events so we can stop the server on exit.
    WidgetsBinding.instance.addObserver(this);
    ref.onDispose(() {
      WidgetsBinding.instance.removeObserver(this);
      // Best-effort: onDispose does not support async, so the graceful
      // SIGTERM + 3 s wait in stop() may not complete before the isolate
      // shuts down. didChangeAppLifecycleState is the primary cleanup path.
      _service.stop();
    });

    // Only start the embedded server if:
    //   1. The binary is present in the bundle.
    //   2. No remote profile is already saved (avoid overwriting user config).
    if (!EmbeddedServerService.isAvailable) {
      return const EmbeddedServerStopped();
    }

    final existingConnection = ref.watch(serverProfileProvider);
    if (existingConnection.value?.isConnected == true) {
      return const EmbeddedServerStopped();
    }

    // Start the server and relay state changes into this notifier.
    await for (final s in _service.start()) {
      state = AsyncData(s);

      if (s is EmbeddedServerReady) {
        // Auto-connect: create a context for the embedded server and activate it.
        final ctx = AssistantContext.create(
          name: 'Local (embedded)',
          serverUrl: s.baseUrl,
          authToken: s.token.isEmpty ? null : s.token,
        );
        await ref.read(contextsProvider.notifier).saveContext(ctx);
        await ref.read(activeContextProvider.notifier).activate(ctx);
        return s;
      }

      if (s is EmbeddedServerError) {
        return s;
      }
    }

    return const EmbeddedServerStopped();
  }

  // -- Public API ------------------------------------------------------------

  /// Stops the embedded server and transitions to [EmbeddedServerStopped].
  Future<void> stop() async {
    await _service.stop();
    state = const AsyncData(EmbeddedServerStopped());
  }

  // -- AppLifecycleObserver --------------------------------------------------

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.detached) {
      _service.stop();
    }
  }
}

/// Provider for [EmbeddedServerNotifier].
final embeddedServerProvider =
    AsyncNotifierProvider<EmbeddedServerNotifier, EmbeddedServerState>(
      EmbeddedServerNotifier.new,
    );
