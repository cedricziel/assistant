import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  group('AssistantUiColors equal the values the moved code renders today', () {
    test('toolOk is Colors.green', () {
      expect(AssistantUiColors.toolOk.toARGB32(), Colors.green.toARGB32());
      expect(AssistantUiColors.toolOk.toARGB32(), 0xFF4CAF50);
    });

    test('toolDenied is Colors.amber', () {
      expect(AssistantUiColors.toolDenied.toARGB32(), Colors.amber.toARGB32());
      expect(AssistantUiColors.toolDenied.toARGB32(), 0xFFFFC107);
    });

    test('warning is the amber-brown the composer renders', () {
      expect(AssistantUiColors.warning.toARGB32(), 0xFFB45309);
    });

    test('fadeMask is opaque white', () {
      expect(AssistantUiColors.fadeMask, const Color(0xFFFFFFFF));
    });
  });

  test('AssistantUiSpacing is a 4 dp scale', () {
    expect(
      [
        AssistantUiSpacing.xs,
        AssistantUiSpacing.sm,
        AssistantUiSpacing.md,
        AssistantUiSpacing.lg,
        AssistantUiSpacing.xl,
        AssistantUiSpacing.xxl,
      ],
      [4, 8, 12, 16, 24, 32],
    );
  });
}
