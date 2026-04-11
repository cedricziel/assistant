import 'dart:io';

import 'package:window_manager/window_manager.dart';

import 'tray_service.dart';

/// Initializes desktop-only features: window manager and tray icon.
///
/// Must be called after [WidgetsFlutterBinding.ensureInitialized].
Future<void> initDesktopFeatures() async {
  if (!Platform.isMacOS) return;

  await windowManager.ensureInitialized();

  const windowOptions = WindowOptions(titleBarStyle: TitleBarStyle.normal);
  await windowManager.waitUntilReadyToShow(windowOptions, () async {
    await windowManager.show();
    await windowManager.focus();
  });

  await TrayService().initIcon();
}
