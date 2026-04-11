import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/models/server_profile.dart';
import '../features/connection/connection_provider.dart';
import '../features/contexts/providers/context_providers.dart';

/// A compact chip widget showing the active server profile.
///
/// - When connected: filled green circle icon + [ServerProfile.displayLabel] + chevron.
/// - When not connected: grey circle + "Not connected".
/// - On tap: opens a modal bottom sheet with connection details and a Disconnect option.
class ServerChip extends ConsumerWidget {
  const ServerChip({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final profile = ref.watch(activeProfileProvider);
    final isConnected = profile != null;

    return InkWell(
      borderRadius: BorderRadius.circular(20),
      onTap: () => _showBottomSheet(context, ref, profile),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.circle,
              size: 10,
              color: isConnected ? Colors.green : Colors.grey,
            ),
            const SizedBox(width: 6),
            Text(
              isConnected ? profile.displayLabel : 'Not connected',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(width: 4),
            const Icon(Icons.chevron_right, size: 16),
          ],
        ),
      ),
    );
  }

  void _showBottomSheet(
    BuildContext context,
    WidgetRef ref,
    ServerProfile? profile,
  ) {
    showModalBottomSheet<void>(
      context: context,
      builder: (sheetContext) {
        return SafeArea(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Text(
                  'Server',
                  style: Theme.of(sheetContext).textTheme.titleMedium,
                ),
              ),
              ListTile(
                title: const Text('URL'),
                subtitle: Text(profile?.baseUrl ?? '–'),
              ),
              ListTile(
                leading: const Icon(Icons.logout),
                title: const Text('Disconnect'),
                onTap: () {
                  ref.read(activeContextProvider.notifier).deactivate();
                  Navigator.of(sheetContext).pop();
                },
              ),
              const SizedBox(height: 8),
            ],
          ),
        );
      },
    );
  }
}
