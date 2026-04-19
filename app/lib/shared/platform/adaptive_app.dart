import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
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
      return CupertinoApp.router(
        title: title,
        theme: const CupertinoThemeData(
          primaryColor: CupertinoColors.systemBlue,
        ),
        routerConfig: routerConfig,
        debugShowCheckedModeBanner: false,
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
