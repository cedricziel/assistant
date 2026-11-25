import '../../shared/theme/assistant_spacing.dart';

/// Horizontal margin that keeps content at most [maxWidth] wide, centred in
/// [availableWidth], and never narrower than the standard page gutter.
double contentSideMargin(double availableWidth, double maxWidth) =>
    ((availableWidth - maxWidth) / 2).clamp(
      AssistantSpacing.lg,
      double.infinity,
    );
