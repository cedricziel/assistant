// flutter_smooth_markdown still takes the framework's ThemeData. Keep the
// deprecated import confined here until it migrates to material_ui. This is
// the only file in the package allowed to import package:flutter/material.dart.
// ignore: deprecated_member_use
import 'package:flutter/material.dart' as legacy;
import 'package:flutter/widgets.dart';

/// The framework `ThemeData` that `MaterialUiCompatibilityBridge` publishes,
/// for APIs such as `MarkdownStyleSheet.fromTheme`.
legacy.ThemeData legacyThemeOf(BuildContext context) =>
    legacy.Theme.of(context);
