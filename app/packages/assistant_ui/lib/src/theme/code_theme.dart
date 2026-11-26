import 'package:material_ui/material_ui.dart';

/// Colours for syntax-highlighted code.
///
/// Each theme owns the [background] its tokens are drawn on, so the contrast
/// test checks the exact pair that renders and an app `ColorScheme` change
/// cannot break it.
///
/// Token colours are ported from highlight.js `a11y-light` and `a11y-dark`
/// (Copyright (c) 2018 Eric Bailey, MIT License), as shipped in
/// `flutter_highlight` 0.7.0 `themes/a11y-*.dart`.
@immutable
class CodeTheme {
  const CodeTheme({
    required this.background,
    required this.foreground,
    required this.tokenColors,
  });

  /// The colour the code body is painted with.
  final Color background;

  /// Colour of plain, unscoped text.
  final Color foreground;

  /// Colour per highlight.js scope.
  final Map<String, Color> tokenColors;

  /// Colour for [scope]; [foreground] when null or not themed.
  Color colorFor(String? scope) => tokenColors[scope] ?? foreground;

  /// Port of `a11y-light`.
  static const light = CodeTheme(
    background: Color(0xFFFEFEFE),
    foreground: Color(0xFF545454),
    tokenColors: {
      'comment': Color(0xFF696969),
      'quote': Color(0xFF696969),
      'variable': Color(0xFFD91E18),
      'template-variable': Color(0xFFD91E18),
      'tag': Color(0xFFD91E18),
      'name': Color(0xFFD91E18),
      'selector-id': Color(0xFFD91E18),
      'selector-class': Color(0xFFD91E18),
      'regexp': Color(0xFFD91E18),
      'deletion': Color(0xFFD91E18),
      'number': Color(0xFFAA5D00),
      'built_in': Color(0xFFAA5D00),
      'builtin-name': Color(0xFFAA5D00),
      'literal': Color(0xFFAA5D00),
      'type': Color(0xFFAA5D00),
      'params': Color(0xFFAA5D00),
      'meta': Color(0xFFAA5D00),
      'link': Color(0xFFAA5D00),
      'attribute': Color(0xFFAA5D00),
      'string': Color(0xFF008000),
      'symbol': Color(0xFF008000),
      'bullet': Color(0xFF008000),
      'addition': Color(0xFF008000),
      'title': Color(0xFF007FAA),
      'section': Color(0xFF007FAA),
      'keyword': Color(0xFF7928A1),
      'selector-tag': Color(0xFF7928A1),
    },
  );

  /// Port of `a11y-dark`.
  static const dark = CodeTheme(
    background: Color(0xFF2B2B2B),
    foreground: Color(0xFFF8F8F2),
    tokenColors: {
      'comment': Color(0xFFD4D0AB),
      'quote': Color(0xFFD4D0AB),
      'variable': Color(0xFFFFA07A),
      'template-variable': Color(0xFFFFA07A),
      'tag': Color(0xFFFFA07A),
      'name': Color(0xFFFFA07A),
      'selector-id': Color(0xFFFFA07A),
      'selector-class': Color(0xFFFFA07A),
      'regexp': Color(0xFFFFA07A),
      'deletion': Color(0xFFFFA07A),
      'number': Color(0xFFF5AB35),
      'built_in': Color(0xFFF5AB35),
      'builtin-name': Color(0xFFF5AB35),
      'literal': Color(0xFFF5AB35),
      'type': Color(0xFFF5AB35),
      'params': Color(0xFFF5AB35),
      'meta': Color(0xFFF5AB35),
      'link': Color(0xFFF5AB35),
      'attribute': Color(0xFFFFD700),
      'string': Color(0xFFABE338),
      'symbol': Color(0xFFABE338),
      'bullet': Color(0xFFABE338),
      'addition': Color(0xFFABE338),
      'title': Color(0xFF00E0E0),
      'section': Color(0xFF00E0E0),
      'keyword': Color(0xFFDCC6E0),
      'selector-tag': Color(0xFFDCC6E0),
    },
  );
}
