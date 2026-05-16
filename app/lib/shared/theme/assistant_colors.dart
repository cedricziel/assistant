import 'package:flutter/material.dart';

/// Named brand + accent + warning colour roles for the assistant app.
///
/// Use these tokens instead of raw `Color(0x...)` or `Colors.X` literals
/// anywhere outside `app/lib/shared/theme/`. Add new tokens here rather
/// than inlining hex values in screens.
abstract class AssistantColors {
  /// Primary brand colour — seeds the Material 3 `ColorScheme.primary`.
  static const Color brand = Color(0xFF1A73E8);

  /// Secondary accent — used for informational chips and complementary
  /// surfaces. Drives `ColorScheme.secondary`.
  static const Color accentSecondary = Color(0xFF6D5BBA);

  /// Tertiary accent — used for success indicators (tool-call OK pill,
  /// successful trace status). Drives `ColorScheme.tertiary`.
  static const Color accentTertiary = Color(0xFF1F8A5C);

  /// Warning amber. Material 3 has no built-in `warning` role, so we
  /// expose it explicitly. Used for `denied` status pills, soft warnings,
  /// and confirmation banners that aren't outright errors.
  static const Color warning = Color(0xFFB45309);
}
