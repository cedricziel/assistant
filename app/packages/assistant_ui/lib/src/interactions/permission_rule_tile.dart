import 'package:material_ui/material_ui.dart';

import '../theme/assistant_ui_spacing.dart';

/// Whether a saved permission rule lets a tool run or blocks it.
enum PermissionRuleEffect {
  allow('Allow'),
  deny('Deny');

  const PermissionRuleEffect(this.label);

  /// Text of the badge.
  final String label;
}

/// One saved tool-permission rule: its effect, the rule text, when it was
/// saved and a revoke action.
///
/// [ownerLabel] names the member who saved the rule; the admin view sets it to
/// list other members' rules. It is display only and does not change what
/// [onRevoke] does. The element never revokes anything itself. While
/// [revoking] is true the revoke button is disabled.
class PermissionRuleTile extends StatelessWidget {
  const PermissionRuleTile({
    super.key,
    required this.effect,
    required this.rule,
    required this.createdAt,
    required this.onRevoke,
    this.ownerLabel,
    this.revoking = false,
  });

  final PermissionRuleEffect effect;
  final String rule;
  final DateTime createdAt;
  final VoidCallback onRevoke;
  final String? ownerLabel;
  final bool revoking;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final owner = ownerLabel;
    final date = MaterialLocalizations.of(context)
        .formatShortDate(createdAt.toLocal());
    final detail = TextStyle(
      color: scheme.onSurfaceVariant,
      fontSize: theme.textTheme.bodySmall?.fontSize,
    );

    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: AssistantUiSpacing.lg,
        vertical: AssistantUiSpacing.sm,
      ),
      child: Row(
        children: [
          _EffectBadge(effect: effect),
          const SizedBox(width: AssistantUiSpacing.md),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  rule,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    fontFamily: 'monospace',
                  ),
                ),
                const SizedBox(height: AssistantUiSpacing.xs),
                DefaultTextStyle.merge(
                  style: detail,
                  child: Row(
                    children: [
                      if (owner != null) ...[
                        Flexible(
                          child: Text(
                            owner,
                            key: const Key('permission_rule_owner'),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontWeight: FontWeight.w600),
                          ),
                        ),
                        const Text(' · '),
                      ],
                      Flexible(
                        child: Text(
                          'Saved $date',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          IconButton(
            tooltip: 'Revoke',
            icon: const Icon(Icons.delete_outline),
            onPressed: revoking ? null : onRevoke,
          ),
        ],
      ),
    );
  }
}

class _EffectBadge extends StatelessWidget {
  const _EffectBadge({required this.effect});

  final PermissionRuleEffect effect;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final allow = effect == PermissionRuleEffect.allow;
    final background = allow ? scheme.tertiaryContainer : scheme.errorContainer;
    final foreground = allow
        ? scheme.onTertiaryContainer
        : scheme.onErrorContainer;

    return DecoratedBox(
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(AssistantUiSpacing.xs),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: AssistantUiSpacing.sm,
          vertical: AssistantUiSpacing.xs,
        ),
        child: Text(
          effect.label,
          style: Theme.of(context).textTheme.labelSmall
              ?.copyWith(color: foreground, fontWeight: FontWeight.w600),
        ),
      ),
    );
  }
}
