import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../../shared/theme/assistant_spacing.dart';
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

    return roles.when(
      loading: () => Scaffold(
        appBar: AppBar(title: const Text('Integrations')),
        body: const Center(child: CircularProgressIndicator.adaptive()),
      ),
      error: (_, _) => Scaffold(
        appBar: AppBar(title: const Text('Integrations')),
        body: Center(
          child: IntegrationStatusView(
            icon: Icons.error_outline,
            iconColor: colors.error,
            message: 'Could not check your role. Try again.',
            action: AdaptiveButton.filled(
              onPressed: () => ref.invalidate(callerRolesProvider),
              child: const Text('Retry'),
            ),
          ),
        ),
      ),
      data: (roles) => roles.isOrgAdmin
          ? const _AdminView()
          : Scaffold(
              appBar: AppBar(title: const Text('Integrations')),
              body: Center(
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
                const Padding(
                  padding: EdgeInsets.fromLTRB(
                    AssistantSpacing.lg,
                    AssistantSpacing.sm,
                    AssistantSpacing.lg,
                    0,
                  ),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: BrowseCatalogButton(label: 'Add integration'),
                  ),
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
