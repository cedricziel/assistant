import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_web_plugins/url_strategy.dart';
import 'package:pwa_update_listener/pwa_update_listener.dart';

import 'features/embedded_server/embedded_server_provider.dart';
import 'router/app_router.dart';
import 'tray/platform_init.dart';
import 'tray/window_handler_platform.dart';

/// Global key used to show SnackBars from outside the widget tree (e.g. the
/// PWA update callback in [main]).
final _scaffoldMessengerKey = GlobalKey<ScaffoldMessengerState>();

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Use /path URLs instead of /#/path. The Rust server's SPA handler serves
  // index.html for every unmatched path, so deep-linking works correctly.
  usePathUrlStrategy();

  // Initialize desktop-only features (window manager + tray icon) on macOS.
  // On web this is a no-op.
  await initDesktopFeatures();

  runApp(
    // PwaUpdateListener detects when a new service worker activates (new
    // version available) and surfaces a SnackBar prompt to reload the app.
    PwaUpdateListener(
      onReady: () {
        _scaffoldMessengerKey.currentState?.showSnackBar(
          SnackBar(
            content: const Text('A new version is available.'),
            action: SnackBarAction(
              label: 'Reload',
              onPressed: reloadPwa,
            ),
            duration: const Duration(days: 1),
          ),
        );
      },
      // Riverpod ProviderScope wraps the entire widget tree.
      child: const ProviderScope(
        child: AssistantApp(),
      ),
    ),
  );
}

/// Root application widget.
class AssistantApp extends ConsumerWidget {
  const AssistantApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Ensure the embedded-server provider is initialised at startup on macOS
    // so the binary starts before the first frame renders.
    ref.watch(embeddedServerProvider);

    final router = ref.watch(routerProvider);

    return MacosWindowCloseHandler(
      child: MaterialApp.router(
        title: 'Assistant',
        scaffoldMessengerKey: _scaffoldMessengerKey,
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(
            seedColor: const Color(0xFF1A73E8),
          ),
          useMaterial3: true,
        ),
        routerConfig: router,
        debugShowCheckedModeBanner: false,
      ),
    );
  }
}
