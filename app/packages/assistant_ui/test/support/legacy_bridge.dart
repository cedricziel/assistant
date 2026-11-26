import 'package:material_ui/material_ui.dart';

/// Publishes the `material_ui` theme and localizations to
/// `flutter_smooth_markdown`, as the app's `LegacyDesignBridge` does.
///
/// Selectable markdown needs the localizations; wrap a test's `MaterialApp`
/// body, or a golden's widget, in it.
Widget withLegacyBridge(Widget child) =>
    // The bridge is deprecated only to mark it as temporary.
    // ignore: deprecated_member_use
    MaterialUiCompatibilityBridge(child: child);
