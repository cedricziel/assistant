import 'package:assistant_api/assistant_api.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../shared/platform/widgets.dart';
import 'browse_catalog_button.dart';
import 'caller_roles.dart';
import 'integration_list_view.dart';
import 'integration_list_view_model.dart';

/// The signed-in user's personal integrations, and the org and space
/// integrations that need their own account.
class ConnectionsScreen extends ConsumerWidget {
  const ConnectionsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final userId = ref.watch(currentUserIdProvider).value;

    return Scaffold(
      appBar: AppBar(title: const Text('Connections')),
      body: IntegrationListView(
        scope: const IntegrationListScope.connections(),
        access: (item) =>
            item.layer == IntegrationLayer.user && item.ownerUserId == userId
            ? const IntegrationRowAccess(toggle: true, delete: true)
            : IntegrationRowAccess.none,
        emptyTitle: 'No connections yet',
        emptyMessage:
            'Personal integrations, and shared ones that ask each person to '
            'connect, show up here.',
        emptyAction: const BrowseCatalogButton(label: 'Browse the catalog'),
      ),
    );
  }
}
