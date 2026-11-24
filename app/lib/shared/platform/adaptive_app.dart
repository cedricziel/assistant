import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:go_router/go_router.dart';

import '../theme/assistant_theme.dart';
import 'platform.dart';

/// Root application widget that renders [CupertinoApp.router] on Apple
/// touch platforms (iPhone, iPad, Mac Catalyst) and [MaterialApp.router]
/// on all other platforms.
///
/// Switching the root widget unlocks platform-native behaviors for free:
/// - iOS edge-swipe-to-go-back page transitions
/// - Bouncing scroll physics
/// - iOS text selection handles
/// - SF Pro system font
///
/// The Cupertino path includes [GlobalMaterialLocalizations] and a
/// [Material] + [Theme] wrapper so that Material widgets (TextField,
/// Scaffold, SnackBar, etc.) continue to work inside the Cupertino shell.
class AdaptiveApp extends StatelessWidget {
  const AdaptiveApp({
    super.key,
    required this.routerConfig,
    this.scaffoldMessengerKey,
    this.title = 'Assistant',
  });

  final GoRouter routerConfig;
  final GlobalKey<ScaffoldMessengerState>? scaffoldMessengerKey;
  final String title;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      final brightness = MediaQuery.platformBrightnessOf(context);
      final materialTheme = brightness == Brightness.dark
          ? assistantDarkTheme()
          : assistantLightTheme();

      return CupertinoApp.router(
        title: title,
        theme: const CupertinoThemeData(
          primaryColor: CupertinoColors.systemBlue,
        ),
        routerConfig: routerConfig,
        debugShowCheckedModeBanner: false,
        localizationsDelegates: GlobalMaterialLocalizations.delegates,
        builder: (context, child) {
          return Theme(
            data: materialTheme,
            child: Material(
              type: MaterialType.transparency,
              child: LegacyDesignBridge(
                child: child ?? const SizedBox.shrink(),
              ),
            ),
          );
        },
      );
    }

    return MaterialApp.router(
      title: title,
      scaffoldMessengerKey: scaffoldMessengerKey,
      theme: assistantLightTheme(),
      darkTheme: assistantDarkTheme(),
      routerConfig: routerConfig,
      debugShowCheckedModeBanner: false,
      builder: (context, child) =>
          LegacyDesignBridge(child: child ?? const SizedBox.shrink()),
    );
  }
}

/// Exposes the app's theme and localizations to dependencies that still
/// build on `package:flutter/material.dart` and `package:flutter/cupertino.dart`
/// (flutter_smooth_markdown, go_router, cupertino_sidebar, window_manager, …).
/// Without it they resolve the framework's default theme.
///
/// Remove once those dependencies have migrated to material_ui/cupertino_ui.
class LegacyDesignBridge extends StatelessWidget {
  const LegacyDesignBridge({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    // The bridges are deprecated only to mark them as temporary.
    // ignore: deprecated_member_use
    return MaterialUiCompatibilityBridge(
      // ignore: deprecated_member_use
      child: CupertinoUiCompatibilityBridge(child: child),
    );
  }
}
