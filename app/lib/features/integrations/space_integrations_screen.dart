import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import '../spaces/space_provider.dart';
import 'browse_catalog_button.dart';
import 'caller_roles.dart';
import 'integration_list_view.dart';
import 'integration_list_view_model.dart';

/// One space's own integrations, plus the org integrations offered to it with
/// an opt-out switch. Space admins edit; members see a read-only view.
class SpaceIntegrationsScreen extends ConsumerWidget {
  const SpaceIntegrationsScreen({super.key, required this.spaceId});

  final String spaceId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final spaces = ref.watch(spacesProvider).value ?? const <SpaceSummary>[];
    final name = spaces.where((s) => s.id == spaceId).firstOrNull?.name;
    final canManage =
        ref.watch(callerRolesProvider).value?.canManageSpace(spaceId) ?? false;
    final muted = TextStyle(
      color: Theme.of(context).colorScheme.onSurfaceVariant,
    );

    return Scaffold(
      appBar: AppBar(
        title: Text(name == null ? 'Space integrations' : '$name integrations'),
      ),
      body: Column(
        children: [
          CatalogActionBar(
            child: canManage
                ? const BrowseCatalogButton(label: 'Add integration')
                : Text(
                    'Only space admins can change these integrations.',
                    style: muted,
                  ),
          ),
          Expanded(
            child: IntegrationListView(
              scope: IntegrationListScope.space(spaceId),
              access: (item) => !canManage
                  ? IntegrationRowAccess.none
                  : item.layer == IntegrationLayer.org
                  ? const IntegrationRowAccess(optOut: true)
                  : const IntegrationRowAccess(toggle: true, delete: true),
              emptyTitle: 'No integrations in this space',
              emptyMessage:
                  'Integrations installed for this space, and the ones the '
                  'organization offers it, appear here.',
              emptyAction: canManage
                  ? const BrowseCatalogButton(label: 'Browse the catalog')
                  : null,
            ),
          ),
        ],
      ),
    );
  }
}
