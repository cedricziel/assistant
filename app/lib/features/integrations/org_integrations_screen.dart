import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import 'browse_catalog_button.dart';
import 'caller_roles.dart';
import 'integration_list_view.dart';
import 'integration_list_view_model.dart';
import 'integration_policy_tab.dart';
import 'integration_status_view.dart';

/// The org's integrations and install policy. Org admins only.
class OrgIntegrationsScreen extends ConsumerWidget {
  const OrgIntegrationsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final roles = ref.watch(callerRolesProvider);
    final colors = Theme.of(context).colorScheme;

    final admin = roles.value?.isOrgAdmin ?? false;
    if (admin) return const _AdminView();

    return Scaffold(
      appBar: AppBar(title: const Text('Integrations')),
      body: roles.when(
        loading: () =>
            const Center(child: CircularProgressIndicator.adaptive()),
        error: (_, _) => Center(
          child: IntegrationStatusView.error(
            context: context,
            message: 'Could not check your role. Try again.',
            onRetry: () => ref.invalidate(callerRolesProvider),
          ),
        ),
        data: (_) => Center(
          child: IntegrationStatusView(
            icon: Icons.lock_outline,
            iconColor: colors.onSurfaceVariant,
            message: 'Only organization admins can manage organization integrations.',
          ),
        ),
      ),
    );
  }
}

class _AdminView extends StatelessWidget {
  const _AdminView();

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 2,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Integrations'),
          bottom: const TabBar(
            tabs: [
              Tab(text: 'Integrations'),
              Tab(text: 'Policy'),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            Column(
              children: [
                const CatalogActionBar(
                  child: BrowseCatalogButton(label: 'Add integration'),
                ),
                Expanded(
                  child: IntegrationListView(
                    scope: const IntegrationListScope.org(),
                    access: (_) =>
                        const IntegrationRowAccess(toggle: true, delete: true),
                    emptyTitle: 'No organization integrations',
                    emptyMessage: 'Install one from the catalog to offer it to every space.',
                    emptyAction: const BrowseCatalogButton(
                      label: 'Browse the catalog',
                    ),
                  ),
                ),
              ],
            ),
            const IntegrationPolicyTab(),
          ],
        ),
      ),
    );
  }
}
