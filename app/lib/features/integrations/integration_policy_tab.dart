import 'package:assistant_api/assistant_api.dart';
import 'package:flutter/foundation.dart' show listEquals;
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'allowlist_picker_sheet.dart';
import 'content_width.dart';
import 'integration_policy_view_model.dart';
import 'integration_status_view.dart';

/// Widest the form grows; wider windows get side margins.
const _maxContentWidth = 640.0;

/// What lower layers may install: catalog mode, allowlist, custom URLs and
/// personal integrations. Org admins only; the server enforces that.
class IntegrationPolicyTab extends ConsumerStatefulWidget {
  const IntegrationPolicyTab({super.key});

  @override
  ConsumerState<IntegrationPolicyTab> createState() =>
      _IntegrationPolicyTabState();
}

class _Draft {
  _Draft(IntegrationPolicyResponse policy)
    : mode = policy.catalogMode,
      allowed = policy.allowedCatalogIds.toList(),
      customUrls = policy.allowCustomUrls,
      userIntegrations = policy.allowUserIntegrations;

  CatalogMode mode;
  List<String> allowed;
  bool customUrls;
  bool userIntegrations;

  bool differsFrom(IntegrationPolicyResponse policy) =>
      mode != policy.catalogMode ||
      customUrls != policy.allowCustomUrls ||
      userIntegrations != policy.allowUserIntegrations ||
      !listEquals(allowed, policy.allowedCatalogIds.toList());
}

class _IntegrationPolicyTabState extends ConsumerState<IntegrationPolicyTab> {
  _Draft? _draft;

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(integrationPolicyViewModelProvider);
    return state.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (_, _) => Center(
        child: IntegrationStatusView.error(
          context: context,
          message:
              'Failed to load the policy. Check your connection and try again.',
          onRetry: () => ref.invalidate(integrationPolicyViewModelProvider),
        ),
      ),
      data: (data) => _form(context, data),
    );
  }

  Widget _form(BuildContext context, IntegrationPolicyState data) {
    final draft = _draft ??= _Draft(data.policy);
    final theme = Theme.of(context);
    final muted = TextStyle(color: theme.colorScheme.onSurfaceVariant);
    final dirty = draft.differsFrom(data.policy);

    return LayoutBuilder(
      builder: (context, constraints) {
        final side = contentSideMargin(constraints.maxWidth, _maxContentWidth);
        return ListView(
          padding: EdgeInsets.symmetric(
            horizontal: side,
            vertical: AssistantSpacing.lg,
          ),
          children: [
            Text('Catalog', style: theme.textTheme.titleMedium),
            const SizedBox(height: AssistantSpacing.xs),
            Text(
              'Which catalog entries spaces and members may install.',
              style: muted,
            ),
            const SizedBox(height: AssistantSpacing.sm),
            Wrap(
              spacing: AssistantSpacing.sm,
              runSpacing: AssistantSpacing.xs,
              children: [
                for (final (mode, label) in [
                  (CatalogMode.all, 'All catalog entries'),
                  (CatalogMode.allowlist, 'Only selected entries'),
                  (CatalogMode.none, 'None'),
                ])
                  ChoiceChip(
                    label: Text(label),
                    selected: draft.mode == mode,
                    onSelected: (_) => setState(() => draft.mode = mode),
                  ),
              ],
            ),
            if (draft.mode == CatalogMode.allowlist)
              ..._allowlist(draft, muted),
            const SizedBox(height: AssistantSpacing.lg),
            AdaptiveSwitchTile(
              title: const Text('Allow custom server URLs'),
              subtitle: Text(
                'Spaces and members may add servers that are not in the catalog.',
                style: muted,
              ),
              value: draft.customUrls,
              onChanged: (on) => setState(() => draft.customUrls = on),
            ),
            AdaptiveSwitchTile(
              title: const Text('Allow personal integrations'),
              subtitle: Text(
                'Members may install integrations only they can use.',
                style: muted,
              ),
              value: draft.userIntegrations,
              onChanged: (on) => setState(() => draft.userIntegrations = on),
            ),
            const SizedBox(height: AssistantSpacing.md),
            Text(
              'Tightening the policy disables existing integrations that no '
              'longer comply. They are kept, not deleted.',
              style: muted,
            ),
            if (data.message != null) ...[
              const SizedBox(height: AssistantSpacing.md),
              Text(
                data.message!,
                style: TextStyle(color: theme.colorScheme.error),
              ),
            ],
            const SizedBox(height: AssistantSpacing.lg),
            Align(
              alignment: Alignment.centerLeft,
              child: AdaptiveButton.filled(
                onPressed: dirty && !data.saving ? () => _save(draft) : null,
                child: data.saving
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Text('Save policy'),
              ),
            ),
          ],
        );
      },
    );
  }

  List<Widget> _allowlist(_Draft draft, TextStyle muted) {
    return [
      const SizedBox(height: AssistantSpacing.lg),
      Text('Allowed entries', style: Theme.of(context).textTheme.titleSmall),
      const SizedBox(height: AssistantSpacing.sm),
      if (draft.allowed.isEmpty)
        Text(
          'Nothing is allowed yet, so nothing can be installed.',
          style: muted,
        )
      else
        Wrap(
          spacing: AssistantSpacing.sm,
          runSpacing: AssistantSpacing.xs,
          children: [
            for (final id in draft.allowed)
              InputChip(
                label: Text(id.split('/').last),
                deleteIcon: const Icon(Icons.cancel, size: 18),
                onDeleted: () => setState(() => draft.allowed.remove(id)),
              ),
          ],
        ),
      Align(
        alignment: Alignment.centerLeft,
        child: AdaptiveButton.plain(
          onPressed: () async {
            final picked = await showAllowlistPicker(context, draft.allowed);
            if (picked != null) setState(() => draft.allowed = picked);
          },
          child: const Text('Add from catalog'),
        ),
      ),
    ];
  }

  Future<void> _save(_Draft draft) async {
    final saved = await ref
        .read(integrationPolicyViewModelProvider.notifier)
        .save(
          catalogMode: draft.mode,
          allowedCatalogIds: draft.allowed,
          allowCustomUrls: draft.customUrls,
          allowUserIntegrations: draft.userIntegrations,
        );
    if (!mounted || !saved) return;
    setState(() => _draft = null);
    showAdaptiveSnackBar(context, 'Policy saved');
  }
}
