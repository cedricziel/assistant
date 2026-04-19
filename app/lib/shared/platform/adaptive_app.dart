import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:go_router/go_router.dart';

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

  static const _seedColor = Color(0xFF1A73E8);

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      final brightness = MediaQuery.platformBrightnessOf(context);
      final materialTheme = ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: _seedColor,
          brightness: brightness,
        ),
        useMaterial3: true,
      );

      return CupertinoApp.router(
        title: title,
        theme: const CupertinoThemeData(
          primaryColor: CupertinoColors.systemBlue,
        ),
        routerConfig: routerConfig,
        debugShowCheckedModeBanner: false,
        localizationsDelegates: const [
          GlobalMaterialLocalizations.delegate,
          GlobalWidgetsLocalizations.delegate,
          GlobalCupertinoLocalizations.delegate,
        ],
        builder: (context, child) {
          return Theme(
            data: materialTheme,
            child: Material(
              type: MaterialType.transparency,
              child: child ?? const SizedBox.shrink(),
            ),
          );
        },
      );
    }

    return MaterialApp.router(
      title: title,
      scaffoldMessengerKey: scaffoldMessengerKey,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: _seedColor),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: _seedColor,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      routerConfig: routerConfig,
      debugShowCheckedModeBanner: false,
    );
  }
}
