import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_web_plugins/url_strategy.dart';

import 'features/contexts/providers/context_providers.dart';
import 'features/embedded_server/embedded_server_provider.dart';
import 'features/notifications/notification_badge_notifier.dart';
import 'features/pwa/pwa_update_service.dart';
import 'router/app_router.dart';
import 'shared/platform/adaptive_app.dart';
import 'tray/platform_init.dart';
import 'tray/tray_platform.dart';
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

  // Initialise context repository before the widget tree is created.
  final contextRepo = await createContextRepository();

  runApp(
    ProviderScope(
      overrides: [contextRepositoryProvider.overrideWithValue(contextRepo)],
      child: const AssistantApp(),
    ),
  );

  // Register for PWA update notifications after the first frame so the
  // ScaffoldMessenger key is bound and can show the SnackBar.
  //
  // Only register on web — reloadPwa is a web-only operation and calling it
  // on iOS/macOS/desktop is dead code at best, a crash risk at worst.
  if (kIsWeb) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      registerPwaUpdateCallback(() {
        _scaffoldMessengerKey.currentState?.showSnackBar(
          SnackBar(
            content: const Text('A new version is available.'),
            action: SnackBarAction(label: 'Reload', onPressed: reloadPwa),
            duration: const Duration(days: 1),
          ),
        );
      });
    });
  }
}

/// Root application widget.
class AssistantApp extends ConsumerStatefulWidget {
  const AssistantApp({super.key});

  @override
  ConsumerState<AssistantApp> createState() => _AssistantAppState();
}

class _AssistantAppState extends ConsumerState<AssistantApp>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    // Wire reactive tray menu once the widget tree (and Ref) is available.
    // Only on macOS — tray_manager is a no-op on other platforms.
    if (!kIsWeb && defaultTargetPlatform == TargetPlatform.macOS) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        TrayService().initMenu(ref);
      });
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      ref.read(notificationBadgeProvider.notifier).clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    // Ensure the embedded-server provider is initialised at startup on macOS
    // so the binary starts before the first frame renders.
    // On iOS/web the embedded server is not available — skip watching it.
    if (!kIsWeb && defaultTargetPlatform == TargetPlatform.macOS) {
      ref.watch(embeddedServerProvider);
    }

    final router = ref.watch(routerProvider);

    return MacosWindowCloseHandler(
      child: AdaptiveApp(
        routerConfig: router,
        scaffoldMessengerKey: _scaffoldMessengerKey,
      ),
    );
  }
}
