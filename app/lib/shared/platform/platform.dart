import 'package:flutter/foundation.dart';

/// Three-bucket platform style that determines which widget library
/// to use for structural chrome.
///
/// - [cupertino]: iOS, iPadOS, and Mac Catalyst ("Designed for iPad").
///   All three report [TargetPlatform.iOS].
/// - [material]: Web and any non-Apple platform.
/// - [macos]: Native macOS app (`flutter build macos`). Kept as a
///   separate bucket for a future macOS-native design pass — falls
///   through to Material rendering for now.
enum AppPlatformStyle {
  cupertino,
  material,
  macos;

  /// True when this style targets Apple touch platforms (iPhone, iPad,
  /// Mac Catalyst).
  bool get isAppleTouch => this == AppPlatformStyle.cupertino;

  /// True when this style targets native macOS.
  bool get isMacOS => this == AppPlatformStyle.macos;

  /// True when this style targets Material (web, Android, etc.).
  bool get isMaterial => this == AppPlatformStyle.material;
}

/// The resolved platform style for the current runtime.
///
/// This is the single source of truth for UI branching — no other file
/// should check [defaultTargetPlatform] for rendering decisions.
AppPlatformStyle get platformStyle {
  if (kIsWeb) return AppPlatformStyle.material;
  if (defaultTargetPlatform == TargetPlatform.macOS) {
    return AppPlatformStyle.macos;
  }
  if (defaultTargetPlatform == TargetPlatform.iOS) {
    return AppPlatformStyle.cupertino;
  }
  return AppPlatformStyle.material;
}

/// Convenience: true on iPhone, iPad, and Mac Catalyst.
bool get isAppleTouch => platformStyle.isAppleTouch;

/// Convenience: true on native macOS.
bool get isMacOS => platformStyle.isMacOS;

/// Convenience: true on web and other non-Apple platforms.
bool get isMaterial => platformStyle.isMaterial;
