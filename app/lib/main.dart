import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_web_plugins/url_strategy.dart';

import 'router/app_router.dart';
import 'tray/platform_init.dart';
import 'tray/window_handler_platform.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Use /path URLs instead of /#/path. The Rust server's SPA handler serves
  // index.html for every unmatched path, so deep-linking works correctly.
  usePathUrlStrategy();

  // Initialize desktop-only features (window manager + tray icon) on macOS.
  // On web this is a no-op.
  await initDesktopFeatures();

  runApp(
    // Riverpod ProviderScope wraps the entire widget tree.
    const ProviderScope(
      child: AssistantApp(),
    ),
  );
}

/// Root application widget.
class AssistantApp extends ConsumerWidget {
  const AssistantApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final router = ref.watch(routerProvider);

    return MacosWindowCloseHandler(
      child: MaterialApp.router(
        title: 'Assistant',
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
