import 'package:assistant_ui/assistant_ui.dart';

/// Spacing scale on a 4 dp base. Replace inline `EdgeInsets.all(12)` /
/// `SizedBox(height: 16)` / similar magic numbers with these tokens so
/// future spacing audits affect every consumer at once.
///
/// The values live in `assistant_ui` so the package and the app cannot drift.
abstract class AssistantSpacing {
  static const double xs = AssistantUiSpacing.xs;
  static const double sm = AssistantUiSpacing.sm;
  static const double md = AssistantUiSpacing.md;
  static const double lg = AssistantUiSpacing.lg;
  static const double xl = AssistantUiSpacing.xl;
  static const double xxl = AssistantUiSpacing.xxl;
}

/// Corner radius scale. Cards, chips, and buttons on the migrated screens
/// SHALL pull from these constants instead of `BorderRadius.circular(12)`
/// literals.
abstract class AssistantRadius {
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
}

/// Elevation scale used for cards, dialogs, and overlays. Values are in
/// logical dp.
abstract class AssistantElevation {
  static const double low = 1;
  static const double medium = 3;
  static const double high = 8;
}
