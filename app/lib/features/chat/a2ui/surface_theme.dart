// ignore: deprecated_member_use
import 'package:flutter/material.dart';

import '../../../shared/theme/assistant_spacing.dart';

/// Gives a surface the app's shapes and padding (ADR-0019 section 8).
///
/// Colours and typography come from the ambient framework theme, which the
/// app's `LegacyDesignBridge` fills from the app theme, in light and dark.
/// A2UI v1.0 has no surface theme, so nothing in the messages restyles this.
class AssistantSurfaceTheme extends StatelessWidget {
  const AssistantSurfaceTheme({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final base = Theme.of(context);
    final field = BorderRadius.circular(AssistantRadius.sm);
    final buttonShape = RoundedRectangleBorder(borderRadius: field);
    final buttonPadding = const EdgeInsets.symmetric(
      horizontal: AssistantSpacing.lg,
      vertical: AssistantSpacing.sm,
    );
    return Theme(
      data: base.copyWith(
        cardTheme: base.cardTheme.copyWith(
          margin: const EdgeInsets.all(AssistantSpacing.xs),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AssistantRadius.md),
          ),
        ),
        filledButtonTheme: FilledButtonThemeData(
          style: FilledButton.styleFrom(
            shape: buttonShape,
            padding: buttonPadding,
          ),
        ),
        outlinedButtonTheme: OutlinedButtonThemeData(
          style: OutlinedButton.styleFrom(
            shape: buttonShape,
            padding: buttonPadding,
          ),
        ),
        textButtonTheme: TextButtonThemeData(
          style: TextButton.styleFrom(
            shape: buttonShape,
            padding: buttonPadding,
          ),
        ),
        inputDecorationTheme: base.inputDecorationTheme.copyWith(
          border: OutlineInputBorder(borderRadius: field),
          contentPadding: const EdgeInsets.symmetric(
            horizontal: AssistantSpacing.md,
            vertical: AssistantSpacing.md,
          ),
        ),
      ),
      // The stock components are framework Material widgets (ink, text
      // fields) and need a framework Material ancestor, which the app's
      // Scaffold does not provide.
      child: Material(type: MaterialType.transparency, child: child),
    );
  }
}
