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
    final (label, swatch, fill) = switch (kind) {
      CatalogAuthKindResponse.none => (
        'No auth',
        colors.tertiary,
        colors.tertiaryContainer,
      ),
      CatalogAuthKindResponse.header => (
        'API key',
        colors.secondary,
        colors.secondaryContainer,
      ),
      CatalogAuthKindResponse.oauth => (
        'OAuth',
        colors.primary,
        colors.primaryContainer,
      ),
      CatalogAuthKindResponse.mixed => (
        'API key or OAuth',
        colors.secondary,
        colors.secondaryContainer,
      ),
      _ => (
        'Auth unknown',
        colors.onSurfaceVariant,
        colors.surfaceContainerHighest,
      ),
    };

    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AssistantSpacing.sm,
        vertical: 3,
      ),
      decoration: BoxDecoration(
        color: fill,
        borderRadius: BorderRadius.circular(AssistantRadius.md),
        border: Border.all(color: swatch.withValues(alpha: 0.4)),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: swatch,
        ),
      ),
    );
  }
}
