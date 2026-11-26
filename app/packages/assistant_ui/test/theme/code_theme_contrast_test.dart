import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_ui/src/theme/code_theme.dart';

/// WCAG 2.x contrast ratio between two opaque colours.
double contrastRatio(Color a, Color b) {
  final la = a.computeLuminance();
  final lb = b.computeLuminance();
  final lighter = la > lb ? la : lb;
  final darker = la > lb ? lb : la;
  return (lighter + 0.05) / (darker + 0.05);
}

void main() {
  const themes = {'light': CodeTheme.light, 'dark': CodeTheme.dark};

  for (final MapEntry(key: name, value: theme) in themes.entries) {
    group('CodeTheme.$name', () {
      test('has opaque colours', () {
        expect(theme.background.a, 1.0);
        expect(theme.foreground.a, 1.0);
        for (final color in theme.tokenColors.values) {
          expect(color.a, 1.0);
        }
      });

      test('plain text reaches 4.5:1 on its own background', () {
        expect(
          contrastRatio(theme.foreground, theme.background),
          greaterThanOrEqualTo(4.5),
        );
      });

      test('every token colour reaches 4.5:1 on its own background', () {
        expect(theme.tokenColors, isNotEmpty);
        for (final MapEntry(key: scope, value: color)
            in theme.tokenColors.entries) {
          expect(
            contrastRatio(color, theme.background),
            greaterThanOrEqualTo(4.5),
            reason: '$name theme, scope "$scope"',
          );
        }
      });

      test('colorFor falls back to the foreground for unknown scopes', () {
        expect(theme.colorFor(null), theme.foreground);
        expect(theme.colorFor('no-such-scope'), theme.foreground);
        expect(theme.colorFor('keyword'), theme.tokenColors['keyword']);
      });
    });
  }

  test('light and dark themes differ in background brightness', () {
    expect(
      CodeTheme.light.background.computeLuminance(),
      greaterThan(CodeTheme.dark.background.computeLuminance()),
    );
  });
}
