import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

/// Typography tokens for the assistant app.
///
/// The theme declares the family names — `Inter` for UI text, `JetBrainsMono`
/// for code surfaces. The actual font binaries are fetched on demand by the
/// `google_fonts` package via [preloadFonts], called from `main()` before
/// the first frame. After preloading, Flutter renders the declared families
/// from the cached binaries; on a cache miss the package silently re-fetches.
abstract class AssistantTypography {
  /// Default UI family name applied to the `TextTheme`.
  static const String fontFamily = 'Inter';

  /// Monospace family name applied to inline code, attribute keys, log lines.
  static const String monoFontFamily = 'JetBrainsMono';

  /// Build a `TextTheme` rooted in the [fontFamily] family.
  static TextTheme material({Brightness brightness = Brightness.light}) {
    final base = ThemeData(brightness: brightness).textTheme;
    return base.apply(fontFamily: fontFamily);
  }

  /// Monospace style for code surfaces.
  static TextStyle get mono =>
      const TextStyle(fontFamily: monoFontFamily, fontSize: 12);

  /// Larger monospace variant (e.g. block code).
  static TextStyle get monoBlock =>
      const TextStyle(fontFamily: monoFontFamily, fontSize: 13);

  /// Preload Inter + JetBrains Mono via `google_fonts`. Call this from
  /// `main()` before `runApp` so the families resolve immediately on first
  /// paint. Safe to call multiple times.
  static Future<void> preloadFonts() async {
    await GoogleFonts.pendingFonts([
      GoogleFonts.inter(),
      GoogleFonts.jetBrainsMono(),
    ]);
  }
}
