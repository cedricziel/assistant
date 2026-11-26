import 'package:assistant_app/shared/theme/assistant_colors.dart';
import 'package:assistant_app/shared/theme/assistant_spacing.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('AssistantColors.warning is the package warning token', () {
    expect(AssistantColors.warning, AssistantUiColors.warning);
  });

  test('every AssistantSpacing value equals its AssistantUiSpacing twin', () {
    expect(AssistantSpacing.xs, AssistantUiSpacing.xs);
    expect(AssistantSpacing.sm, AssistantUiSpacing.sm);
    expect(AssistantSpacing.md, AssistantUiSpacing.md);
    expect(AssistantSpacing.lg, AssistantUiSpacing.lg);
    expect(AssistantSpacing.xl, AssistantUiSpacing.xl);
    expect(AssistantSpacing.xxl, AssistantUiSpacing.xxl);
  });
}
