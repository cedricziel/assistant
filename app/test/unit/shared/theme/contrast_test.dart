import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/shared/theme/assistant_theme.dart';

/// WCAG 2.1 relative luminance.
double _luminance(Color c) {
  double channel(double s) {
    return s <= 0.03928
        ? s / 12.92
        : math.pow((s + 0.055) / 1.055, 2.4) as double;
  }

  // Color.r/g/b in modern Flutter are sRGB doubles in [0, 1].
  final r = channel(c.r);
  final g = channel(c.g);
  final b = channel(c.b);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

double contrastRatio(Color a, Color b) {
  final la = _luminance(a);
  final lb = _luminance(b);
  final lighter = math.max(la, lb);
  final darker = math.min(la, lb);
  return (lighter + 0.05) / (darker + 0.05);
}

void main() {
  group('Theme contrast — body text (AA: 4.5:1)', () {
    test('light theme: onSurface vs surface passes AA', () {
      final scheme = assistantLightTheme().colorScheme;
      expect(
        contrastRatio(scheme.onSurface, scheme.surface),
        greaterThanOrEqualTo(4.5),
      );
    });

    test('dark theme: onSurface vs surface passes AA', () {
      final scheme = assistantDarkTheme().colorScheme;
      expect(
        contrastRatio(scheme.onSurface, scheme.surface),
        greaterThanOrEqualTo(4.5),
      );
    });

    test('light theme: onSurface vs surfaceContainerLowest passes AA', () {
      final scheme = assistantLightTheme().colorScheme;
      expect(
        contrastRatio(scheme.onSurface, scheme.surfaceContainerLowest),
        greaterThanOrEqualTo(4.5),
      );
    });

    test('dark theme: onSurface vs surfaceContainerLowest passes AA', () {
      final scheme = assistantDarkTheme().colorScheme;
      expect(
        contrastRatio(scheme.onSurface, scheme.surfaceContainerLowest),
        greaterThanOrEqualTo(4.5),
      );
    });
  });

  group('Theme contrast — pills / large text (AA: 3:1)', () {
    test('light theme: onPrimary vs primary passes large-text AA', () {
      final scheme = assistantLightTheme().colorScheme;
      expect(
        contrastRatio(scheme.onPrimary, scheme.primary),
        greaterThanOrEqualTo(3),
      );
    });

    test('dark theme: onPrimary vs primary passes large-text AA', () {
      final scheme = assistantDarkTheme().colorScheme;
      expect(
        contrastRatio(scheme.onPrimary, scheme.primary),
        greaterThanOrEqualTo(3),
      );
    });

    test('light theme: error text vs surface passes large-text AA', () {
      final scheme = assistantLightTheme().colorScheme;
      expect(
        contrastRatio(scheme.error, scheme.surface),
        greaterThanOrEqualTo(3),
      );
    });

    test('dark theme: tertiary text vs surface passes large-text AA', () {
      final scheme = assistantDarkTheme().colorScheme;
      expect(
        contrastRatio(scheme.tertiary, scheme.surface),
        greaterThanOrEqualTo(3),
      );
    });
  });
}
