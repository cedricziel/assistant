import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/theme/assistant_colors.dart';
import 'package:assistant_app/shared/theme/assistant_spacing.dart';
import 'package:assistant_app/shared/theme/assistant_theme.dart';
import 'package:assistant_app/shared/theme/assistant_typography.dart';

void main() {
  group('assistantLightTheme()', () {
    final theme = assistantLightTheme();

    test('uses Material 3', () {
      expect(theme.useMaterial3, isTrue);
    });

    test('default text family starts with Inter', () {
      expect(theme.textTheme.bodyLarge?.fontFamily, startsWith('Inter'));
    });

    test('color scheme tertiary equals AssistantColors.accentTertiary', () {
      expect(theme.colorScheme.tertiary, AssistantColors.accentTertiary);
    });

    test('color scheme secondary equals AssistantColors.accentSecondary', () {
      expect(theme.colorScheme.secondary, AssistantColors.accentSecondary);
    });
  });

  group('assistantDarkTheme()', () {
    final theme = assistantDarkTheme();

    test('uses Material 3', () {
      expect(theme.useMaterial3, isTrue);
    });

    test('color scheme brightness is dark', () {
      expect(theme.colorScheme.brightness, Brightness.dark);
    });

    test('color scheme tertiary equals AssistantColors.accentTertiary', () {
      expect(theme.colorScheme.tertiary, AssistantColors.accentTertiary);
    });
  });

  group('AssistantTypography', () {
    test('mono style uses JetBrainsMono', () {
      expect(AssistantTypography.mono.fontFamily, startsWith('JetBrainsMono'));
    });
  });

  group('AssistantSpacing', () {
    test('defines the documented scale', () {
      expect(AssistantSpacing.xs, 4);
      expect(AssistantSpacing.sm, 8);
      expect(AssistantSpacing.md, 12);
      expect(AssistantSpacing.lg, 16);
      expect(AssistantSpacing.xl, 24);
      expect(AssistantSpacing.xxl, 32);
    });
  });

  group('AssistantRadius', () {
    test('defines small/medium/large radii', () {
      expect(AssistantRadius.sm, 8);
      expect(AssistantRadius.md, 12);
      expect(AssistantRadius.lg, 16);
    });
  });

  group('AssistantColors', () {
    test('exposes brand + accent roles + warning', () {
      expect(AssistantColors.brand, isA<Color>());
      expect(AssistantColors.accentSecondary, isA<Color>());
      expect(AssistantColors.accentTertiary, isA<Color>());
      expect(AssistantColors.warning, isA<Color>());
    });
  });
}
