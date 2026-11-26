import 'package:material_ui/material_ui.dart';

/// Colour tokens whose values equal what the moved chat code renders today,
/// so extracting a widget moves no pixel.
///
/// This directory is the only place in the package where raw colours may
/// appear (`app/scripts/check_no_raw_colors.sh`).
abstract class AssistantUiColors {
  /// Warning amber; the same value as the app's `AssistantColors.warning`.
  static const Color warning = Color(0xFFB45309);

  /// Successful tool call; equals `Colors.green`.
  static const Color toolOk = Color(0xFF4CAF50);

  /// Denied tool call; equals `Colors.amber`. Deliberately not [warning]:
  /// swapping them would change rendered pixels.
  static const Color toolDenied = Color(0xFFFFC107);

  /// Opaque stop of the fade shader. Only its alpha is used.
  static const Color fadeMask = Color(0xFFFFFFFF);
}
