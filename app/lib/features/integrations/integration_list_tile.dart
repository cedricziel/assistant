import 'package:assistant_api/assistant_api.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'catalog_entry_tile.dart';

/// Shown on the disabled Connect action until credential setup ships.
const connectUnavailableHint = 'Credential setup arrives in a later update.';

/// One installed integration in a layer list.
///
/// Controls appear only for the callbacks given, so a read-only view passes
/// none. [onEnabledChanged] and [onOptedOutChanged] are alternatives: the
/// first toggles an integration the viewer manages, the second a space's
/// opt-out from an org integration.
class IntegrationListTile extends StatelessWidget {
  const IntegrationListTile({
    super.key,
    required this.integration,
    this.busy = false,
    this.onEnabledChanged,
    this.onDelete,
    this.optedOut = false,
    this.onOptedOutChanged,
    this.actsAs,
  });

  final IntegrationSummary integration;

  /// A request for this integration is in flight; its controls are disabled.
  final bool busy;

  final ValueChanged<bool>? onEnabledChanged;
  final VoidCallback? onDelete;

  /// The space opted out of this org-layer integration.
  final bool optedOut;

  /// Called with `true` to opt out, `false` to opt back in.
  final ValueChanged<bool>? onOptedOutChanged;

  /// The account a shared credential acts as, when the server says.
  final String? actsAs;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final muted = TextStyle(fontSize: 12, color: colors.onSurfaceVariant);
    final toggle = onEnabledChanged;
    final optOut = onOptedOutChanged;
    final delete = onDelete;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(AssistantSpacing.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                CatalogEntryIcon(name: integration.name),
                const SizedBox(width: AssistantSpacing.md),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        integration.name,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 14,
                        ),
                      ),
                      Text(
                        '${_layerLabel(integration.layer)} · ${integration.slug}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: muted,
                      ),
                    ],
                  ),
                ),
                if (toggle != null)
                  Semantics(
                    label: 'Enable ${integration.name}',
                    child: AdaptiveSwitch(
                      value: integration.enabled,
                      onChanged: busy ? null : toggle,
                    ),
                  ),
                if (delete != null)
                  IconButton(
                    tooltip: 'Remove',
                    icon: Icon(Icons.delete_outline, color: colors.error),
                    onPressed: busy ? null : delete,
                  ),
              ],
            ),
            const SizedBox(height: AssistantSpacing.sm),
            Wrap(
              spacing: AssistantSpacing.sm,
              runSpacing: AssistantSpacing.xs,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                IntegrationStatusChip(status: integration.status),
                Text(
                  integration.credentialMode == CredentialMode.perUser
                      ? 'Each person connects their own account'
                      : 'One shared credential',
                  style: muted,
                ),
              ],
            ),
            if (actsAs != null) ...[
              const SizedBox(height: AssistantSpacing.xs),
              Text('Acts as $actsAs', style: muted),
            ],
            if (optOut != null) ...[
              const SizedBox(height: AssistantSpacing.xs),
              Row(
                children: [
                  const Expanded(child: Text('Use in this space')),
                  Semantics(
                    label: 'Use ${integration.name} in this space',
                    child: AdaptiveSwitch(
                      value: !optedOut,
                      onChanged: busy ? null : (use) => optOut(!use),
                    ),
                  ),
                ],
              ),
            ],
            if (integration.status == IntegrationStatus.needsConnection) ...[
              const SizedBox(height: AssistantSpacing.sm),
              const Tooltip(
                message: connectUnavailableHint,
                child: AdaptiveButton.filled(
                  onPressed: null,
                  child: Text('Connect'),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

String _layerLabel(IntegrationLayer layer) => switch (layer) {
  IntegrationLayer.org => 'Organization',
  IntegrationLayer.space => 'Space',
  IntegrationLayer.user => 'Personal',
  _ => 'Unknown layer',
};

/// Pill naming an integration's status for the viewer.
class IntegrationStatusChip extends StatelessWidget {
  const IntegrationStatusChip({super.key, required this.status});

  final IntegrationStatus status;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final (label, text, fill) = switch (status) {
      IntegrationStatus.ready => (
        'Ready',
        colors.onTertiaryContainer,
        colors.tertiaryContainer,
      ),
      IntegrationStatus.needsConnection => (
        'Needs connection',
        colors.onSecondaryContainer,
        colors.secondaryContainer,
      ),
      IntegrationStatus.authExpired => (
        'Sign-in expired',
        colors.onErrorContainer,
        colors.errorContainer,
      ),
      IntegrationStatus.unreachable => (
        'Unreachable',
        colors.onErrorContainer,
        colors.errorContainer,
      ),
      IntegrationStatus.blockedByPolicy => (
        'Blocked by policy',
        colors.onErrorContainer,
        colors.errorContainer,
      ),
      IntegrationStatus.disabled => (
        'Disabled',
        colors.onSurfaceVariant,
        colors.surfaceContainerHighest,
      ),
      _ => ('Unknown', colors.onSurfaceVariant, colors.surfaceContainerHighest),
    };

    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AssistantSpacing.sm,
        vertical: 3,
      ),
      decoration: BoxDecoration(
        color: fill,
        borderRadius: BorderRadius.circular(AssistantRadius.md),
        border: Border.all(color: text.withValues(alpha: 0.4)),
      ),
      child: Text(
        label,
        style: TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w600,
          color: text,
        ),
      ),
    );
  }
}
