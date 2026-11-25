import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
import 'integration_list_tile.dart';
import 'integration_list_view_model.dart';
import 'integration_status_view.dart';

/// Widest the list grows; wider windows get side margins.
const _maxContentWidth = 840.0;

/// What the viewer may do with one integration.
class IntegrationRowAccess {
  const IntegrationRowAccess({
    this.toggle = false,
    this.delete = false,
    this.optOut = false,
  });

  /// Read-only.
  static const none = IntegrationRowAccess();

  /// Enable or disable it.
  final bool toggle;

  /// Remove it.
  final bool delete;

  /// Opt the space out of, or back in to, an org-layer integration.
  final bool optOut;
}

/// The integrations of one [IntegrationListScope] with enable, opt-out and
/// remove controls, as far as [access] allows them.
class IntegrationListView extends ConsumerWidget {
  const IntegrationListView({
    super.key,
    required this.scope,
    required this.access,
    required this.emptyTitle,
    required this.emptyMessage,
    this.emptyAction,
  });

  final IntegrationListScope scope;

  /// What the viewer may do with each integration.
  final IntegrationRowAccess Function(IntegrationSummary) access;

  final String emptyTitle;
  final String emptyMessage;
  final Widget? emptyAction;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final provider = integrationListViewModelProvider(scope);
    final state = ref.watch(provider);
    final vm = ref.read(provider.notifier);
    ref.listen(provider.select((s) => s.value?.message), (_, message) {
      if (message == null) return;
      showAdaptiveSnackBar(
        context,
        message,
        duration: const Duration(seconds: 4),
      );
      vm.clearMessage();
    });

    final colors = Theme.of(context).colorScheme;
    return state.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (error, _) => Center(
        child: IntegrationStatusView(
          icon: Icons.error_outline,
          iconColor: colors.error,
          message: 'Failed to load integrations. Check your connection and try again.',
          action: AdaptiveButton.filled(
            onPressed: () => ref.invalidate(provider),
            child: const Text('Retry'),
          ),
        ),
      ),
      data: (data) {
        if (data.rows.isEmpty) {
          return Center(
            child: IntegrationStatusView(
              icon: Icons.hub_outlined,
              iconColor: colors.onSurfaceVariant,
              title: emptyTitle,
              message: emptyMessage,
              action: emptyAction,
            ),
          );
        }
        return LayoutBuilder(
          builder: (context, constraints) {
            final side = ((constraints.maxWidth - _maxContentWidth) / 2).clamp(
              AssistantSpacing.lg,
              double.infinity,
            );
            return ListView.separated(
              padding: EdgeInsets.symmetric(
                horizontal: side,
                vertical: AssistantSpacing.lg,
              ),
              itemCount: data.rows.length,
              separatorBuilder: (_, _) =>
                  const SizedBox(height: AssistantSpacing.sm),
              itemBuilder: (context, index) {
                final row = data.rows[index];
                final item = row.integration;
                final allowed = access(item);
                return IntegrationListTile(
                  key: ValueKey(item.id),
                  integration: item,
                  busy: data.busyIds.contains(item.id),
                  optedOut: row.optedOut,
                  onEnabledChanged: allowed.toggle
                      ? (enabled) => vm.setEnabled(item.id, enabled)
                      : null,
                  onOptedOutChanged: allowed.optOut
                      ? (out) => vm.setOptedOut(item.id, out)
                      : null,
                  onDelete: allowed.delete
                      ? () => _confirmDelete(context, vm, item)
                      : null,
                );
              },
            );
          },
        );
      },
    );
  }

  Future<void> _confirmDelete(
    BuildContext context,
    IntegrationListViewModel vm,
    IntegrationSummary item,
  ) async {
    final confirmed = await showAdaptiveConfirmDialog(
      context: context,
      title: 'Remove ${item.name}?',
      content:
          'Its stored credentials are deleted and its tools disappear from '
          'every conversation that used it.',
      confirmLabel: 'Remove',
      isDestructive: true,
    );
    if (confirmed) await vm.delete(item.id);
  }
}
