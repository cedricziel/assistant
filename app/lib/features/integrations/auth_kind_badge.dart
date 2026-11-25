import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';

/// Pill that tells how a catalog entry authenticates.
class AuthKindBadge extends StatelessWidget {
  const AuthKindBadge({super.key, required this.kind});

  final CatalogAuthKindResponse kind;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    // Text uses the on-container role so it stays readable in both themes.
    final (label, text, fill) = switch (kind) {
      CatalogAuthKindResponse.none => (
        'No auth',
        colors.onTertiaryContainer,
        colors.tertiaryContainer,
      ),
      CatalogAuthKindResponse.header => (
        'API key',
        colors.onSecondaryContainer,
        colors.secondaryContainer,
      ),
      CatalogAuthKindResponse.oauth => (
        'OAuth',
        colors.onPrimaryContainer,
        colors.primaryContainer,
      ),
      CatalogAuthKindResponse.mixed => (
        'API key or OAuth',
        colors.onSecondaryContainer,
        colors.secondaryContainer,
      ),
      _ => (
        'Auth unknown',
        colors.onSurfaceVariant,
        colors.surfaceContainerHighest,
      ),
    };

    return StatusPill(label: label, textColor: text, fillColor: fill);
  }
}

/// Small bordered pill with a coloured fill, shared by the badges here.
class StatusPill extends StatelessWidget {
  const StatusPill({
    super.key,
    required this.label,
    required this.textColor,
    required this.fillColor,
  });

  final String label;

  /// Also tints the border; use the on-container colour of [fillColor].
  final Color textColor;
  final Color fillColor;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AssistantSpacing.sm,
        vertical: 3,
      ),
      decoration: BoxDecoration(
        color: fillColor,
        borderRadius: BorderRadius.circular(AssistantRadius.md),
        border: Border.all(color: textColor.withValues(alpha: 0.4)),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: textColor,
        ),
      ),
    );
  }
}
