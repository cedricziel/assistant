import 'package:assistant_api/assistant_api.dart'
    show ToolPermissionRuleSummary;
import 'package:assistant_ui/assistant_ui.dart'
    show PermissionRuleEffect, PermissionRuleTile;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../shared/platform/widgets.dart';
import 'tool_permission_rules_provider.dart';

/// Settings section listing the user's saved tool approvals in the current
/// space, grouped as Allowed and Denied, each with a revoke action.
class ToolApprovalsSection extends ConsumerWidget {
  const ToolApprovalsSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final rules = ref.watch(toolPermissionRulesProvider);

    return AdaptiveListSection(
      header: const Text('Tool approvals'),
      children: [
        const _Note(
          'Saved approvals apply to all of your turns in this space, '
          'whichever chat or interface they run from.',
        ),
        ...rules.when(
          skipLoadingOnReload: true,
          loading: () => const [
            Padding(
              padding: EdgeInsets.all(16),
              child: Center(child: CircularProgressIndicator.adaptive()),
            ),
          ],
          error: (_, _) => [
            AdaptiveListTile(
              leading: const Icon(Icons.error_outline),
              title: const Text('Could not load saved approvals'),
              trailing: TextButton(
                onPressed: () => ref.invalidate(toolPermissionRulesProvider),
                child: const Text('Retry'),
              ),
            ),
          ],
          data: (list) => list.isEmpty
              ? const [
                  AdaptiveListTile(
                    leading: Icon(Icons.verified_user_outlined),
                    title: Text('No approvals saved'),
                    subtitle: Text(
                      'Choose "Always allow" on a tool approval in chat to '
                      'save one here.',
                    ),
                  ),
                ]
              : [
                  ..._group(context, ref, 'Allowed', 'allow', list),
                  ..._group(context, ref, 'Denied', 'deny', list),
                ],
        ),
      ],
    );
  }

  List<Widget> _group(
    BuildContext context,
    WidgetRef ref,
    String title,
    String effect,
    List<ToolPermissionRuleSummary> all,
  ) {
    final rules = all.where((r) => r.effect == effect).toList();
    if (rules.isEmpty) return const [];
    return [
      _GroupLabel(title),
      for (final rule in rules)
        PermissionRuleTile(
          key: ValueKey(rule.id),
          effect: effect == 'allow'
              ? PermissionRuleEffect.allow
              : PermissionRuleEffect.deny,
          rule: rule.rule,
          createdAt: DateTime.tryParse(rule.createdAt) ?? DateTime.now(),
          onRevoke: () => _confirmRevoke(context, ref, rule),
        ),
    ];
  }

  Future<void> _confirmRevoke(
    BuildContext context,
    WidgetRef ref,
    ToolPermissionRuleSummary rule,
  ) async {
    final messenger = ScaffoldMessenger.maybeOf(context);
    final confirmed = await showAdaptiveConfirmDialog(
      context: context,
      title: 'Revoke approval?',
      content:
          '${rule.rule} will ask for approval again the next time it is '
          'needed.',
      confirmLabel: 'Revoke',
      isDestructive: true,
    );
    if (!confirmed) return;
    try {
      await ref.read(toolPermissionRulesProvider.notifier).revoke(rule.id);
    } catch (_) {
      messenger?.showSnackBar(
        const SnackBar(content: Text('Could not revoke the approval.')),
      );
    }
  }
}

class _Note extends StatelessWidget {
  const _Note(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 8),
      child: Text(
        text,
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _GroupLabel extends StatelessWidget {
  const _GroupLabel(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
      child: Text(
        text,
        style: theme.textTheme.labelMedium?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}
