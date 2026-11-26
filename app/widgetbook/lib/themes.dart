import 'package:material_ui/material_ui.dart';
import 'package:widgetbook/widgetbook.dart';

/// Brand seed and accent roles of the app's theme (`assistant_theme.dart`).
/// Duplicated because the gallery cannot import the app.
const _seed = Color(0xFF1A73E8);
const _secondary = Color(0xFF6D5BBA);
const _tertiary = Color(0xFF1F8A5C);

ThemeData _theme(Brightness brightness) {
  final scheme = ColorScheme.fromSeed(
    seedColor: _seed,
    brightness: brightness,
  ).copyWith(secondary: _secondary, tertiary: _tertiary);
  return ThemeData(
    useMaterial3: true,
    colorScheme: scheme,
    brightness: brightness,
  );
}

/// Light and dark themes for the elements.
///
/// `MaterialThemeAddon` is not used: it builds `ThemeData` from
/// `package:flutter/material.dart`, which the elements, written against
/// `material_ui`, do not read.
final ThemeAddon<ThemeData> kThemeAddon = ThemeAddon<ThemeData>(
  themes: [
    WidgetbookTheme(name: 'Light', data: _theme(Brightness.light)),
    WidgetbookTheme(name: 'Dark', data: _theme(Brightness.dark)),
  ],
  themeBuilder: (context, theme, child) => Theme(
    data: theme,
    child: ColoredBox(
      color: theme.scaffoldBackgroundColor,
      child: DefaultTextStyle(style: theme.textTheme.bodyMedium!, child: child),
    ),
  ),
);

/// Hosts each use-case in a `material_ui` app so elements find the
/// localizations, `Material` and `Navigator` they expect.
Widget materialUiAppBuilder(BuildContext context, Widget child) => MaterialApp(
  debugShowCheckedModeBanner: false,
  home: Material(child: child),
);
