import 'package:assistant_api/assistant_api.dart'
    show ToolPermissionRuleSummary, UserSummary;
import 'package:assistant_ui/assistant_ui.dart'
    show PermissionRuleEffect, PermissionRuleTile;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../../shared/platform/widgets.dart';
import '../account/account_provider.dart';
import '../admin/admin_providers.dart';
import '../integrations/caller_roles.dart';
import '../spaces/space_provider.dart';
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
        const MembersApprovalsGroup(),
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
          effect: _effect(rule),
          rule: rule.rule,
          createdAt: _createdAt(rule),
          onRevoke: () => _confirmRevoke(
            context,
            rule,
            revoke: () =>
                ref.read(toolPermissionRulesProvider.notifier).revoke(rule.id),
          ),
        ),
    ];
  }
}

/// The other members' saved rules in the selected space.
///
/// Shown only to a space or org admin, and only for rules the caller does not
/// own. Members and viewers never watch the space provider, so opening the
/// section never sends a `scope=space` request for them. A `403` (or any other
/// failure) hides the group rather than replacing the caller's own rules.
class MembersApprovalsGroup extends ConsumerWidget {
  const MembersApprovalsGroup({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final spaceId = ref.watch(spaceSelectionProvider).spaceId;
    final roles = ref.watch(callerRolesProvider).value;
    if (spaceId == null || roles == null || !roles.canManageSpace(spaceId)) {
      return const SizedBox.shrink();
    }

    final callerId = ref.watch(currentUserIdProvider).value;
    final names = {
      for (final user
          in ref.watch(adminUsersProvider).value ?? const <UserSummary>[])
        user.id: user.name,
    };
    final rules = ref.watch(spaceToolPermissionRulesProvider).value;
    if (callerId == null || rules == null) return const SizedBox.shrink();

    final others = rules.where((r) => r.userId != callerId).toList();
    if (others.isEmpty) return const SizedBox.shrink();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        const _GroupLabel("Members' approvals"),
        for (final rule in others)
          PermissionRuleTile(
            key: ValueKey('member-${rule.id}'),
            effect: _effect(rule),
            rule: rule.rule,
            ownerLabel: _ownerLabel(names, rule),
            createdAt: _createdAt(rule),
            onRevoke: () => _confirmRevoke(
              context,
              rule,
              ownerLabel: _ownerLabel(names, rule),
              revoke: () => ref
                  .read(spaceToolPermissionRulesProvider.notifier)
                  .revoke(rule.id),
            ),
          ),
      ],
    );
  }
}

String _ownerLabel(Map<String, String> names, ToolPermissionRuleSummary rule) {
  final name = names[rule.userId];
  return name == null || name.isEmpty ? rule.userId : name;
}

PermissionRuleEffect _effect(ToolPermissionRuleSummary rule) =>
    rule.effect == 'allow'
    ? PermissionRuleEffect.allow
    : PermissionRuleEffect.deny;

DateTime _createdAt(ToolPermissionRuleSummary rule) =>
    DateTime.tryParse(rule.createdAt) ?? DateTime.now();

Future<void> _confirmRevoke(
  BuildContext context,
  ToolPermissionRuleSummary rule, {
  required Future<void> Function() revoke,
  String? ownerLabel,
}) async {
  final messenger = ScaffoldMessenger.maybeOf(context);
  final subject = ownerLabel == null ? rule.rule : "$ownerLabel's ${rule.rule}";
  final confirmed = await showAdaptiveConfirmDialog(
    context: context,
    title: 'Revoke approval?',
    content: '$subject will ask for approval again the next time it is needed.',
    confirmLabel: 'Revoke',
    isDestructive: true,
  );
  if (!confirmed) return;
  try {
    await revoke();
  } catch (_) {
    messenger?.showSnackBar(
      const SnackBar(content: Text('Could not revoke the approval.')),
    );
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
