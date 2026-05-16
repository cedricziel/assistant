import 'package:flutter/material.dart';

import 'assistant_colors.dart';
import 'assistant_typography.dart';

/// Build the light `ThemeData` for the assistant app.
///
/// Uses `ColorScheme.fromSeed` with explicit overrides for the secondary
/// and tertiary accent roles. Typography is applied via [AssistantTypography].
ThemeData assistantLightTheme() => _build(Brightness.light);

/// Build the dark `ThemeData` for the assistant app.
ThemeData assistantDarkTheme() => _build(Brightness.dark);

ThemeData _build(Brightness brightness) {
  final scheme =
      ColorScheme.fromSeed(
        seedColor: AssistantColors.brand,
        brightness: brightness,
      ).copyWith(
        secondary: AssistantColors.accentSecondary,
        tertiary: AssistantColors.accentTertiary,
      );

  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    brightness: brightness,
    textTheme: AssistantTypography.material(brightness: brightness),
  );
}
