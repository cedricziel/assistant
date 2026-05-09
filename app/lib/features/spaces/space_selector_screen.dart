import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../router/app_router.dart';
import 'space_provider.dart';

/// Screen shown after login for users to select their org and space.
///
/// Two-step flow:
/// 1. Select an organization from the list
/// 2. Select a space within that organization
///
/// On completion, navigates to chat (or onboarding if no persona exists).
class SpaceSelectorScreen extends ConsumerWidget {
  const SpaceSelectorScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selection = ref.watch(spaceSelectionProvider);

    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 480),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(32),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  selection.orgId == null
                      ? 'Select organization'
                      : 'Select space',
                  style: Theme.of(context).textTheme.headlineMedium,
                ),
                const SizedBox(height: 8),
                Text(
                  selection.orgId == null
                      ? 'Choose the organization you want to work in.'
                      : 'Choose a space within ${selection.orgName}.',
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
                const SizedBox(height: 24),

                // Step 1: Org selection.
                if (selection.orgId == null) _OrgList(),

                // Step 2: Space selection.
                if (selection.orgId != null) ...[
                  // Back button to change org.
                  Align(
                    alignment: Alignment.centerLeft,
                    child: TextButton.icon(
                      onPressed: () {
                        ref.read(spaceSelectionProvider.notifier).clear();
                      },
                      icon: const Icon(Icons.arrow_back, size: 18),
                      label: const Text('Change organization'),
                    ),
                  ),
                  const SizedBox(height: 16),
                  _SpaceList(),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _OrgList extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final orgsAsync = ref.watch(orgsProvider);

    return orgsAsync.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (err, _) => _ErrorCard(
        message: 'Failed to load organizations: $err',
        onRetry: () => ref.invalidate(orgsProvider),
      ),
      data: (orgs) {
        if (orgs.isEmpty) {
          return const _EmptyCard(
            message:
                'No organizations found.\n'
                'Ask an administrator to invite you to an organization.',
          );
        }

        // If only one org, auto-select it.
        if (orgs.length == 1) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            final notifier = ref.read(spaceSelectionProvider.notifier);
            final current = ref.read(spaceSelectionProvider);
            if (current.orgId == null) {
              notifier.selectOrg(
                orgId: orgs.first.id,
                orgName: orgs.first.name,
              );
            }
          });
          return const Center(child: CircularProgressIndicator.adaptive());
        }

        return Column(
          children: [
            for (final org in orgs)
              Card(
                child: ListTile(
                  title: Text(org.name),
                  subtitle: Text(org.slug),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () {
                    ref
                        .read(spaceSelectionProvider.notifier)
                        .selectOrg(orgId: org.id, orgName: org.name);
                  },
                ),
              ),
          ],
        );
      },
    );
  }
}

class _SpaceList extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final spacesAsync = ref.watch(spacesProvider);

    return spacesAsync.when(
      loading: () => const Center(child: CircularProgressIndicator.adaptive()),
      error: (err, _) => _ErrorCard(
        message: 'Failed to load spaces: $err',
        onRetry: () => ref.invalidate(spacesProvider),
      ),
      data: (spaces) {
        if (spaces.isEmpty) {
          return const _EmptyCard(
            message:
                'No spaces available in this organization.\n'
                'Ask a space admin to add you as a member.',
          );
        }

        final selection = ref.watch(spaceSelectionProvider);

        // First-time auto-select on a single-space org: pick it for the user
        // and bounce to /chat. Only fires when nothing is selected yet —
        // revisits with `spaceId` already set fall through to the list below
        // so the user is never stuck on an indefinite spinner.
        if (spaces.length == 1 && selection.spaceId == null) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            // Re-check inside the callback in case state changed in between.
            if (ref.read(spaceSelectionProvider).spaceId == null) {
              ref
                  .read(spaceSelectionProvider.notifier)
                  .selectSpace(
                    spaceId: spaces.first.id,
                    spaceName: spaces.first.name,
                  );
              GoRouter.of(context).go(AppRoutes.chat);
            }
          });
          return const Center(child: CircularProgressIndicator.adaptive());
        }

        return Column(
          children: [
            for (final space in spaces)
              Card(
                color: space.id == selection.spaceId
                    ? Theme.of(context).colorScheme.primaryContainer
                    : null,
                child: ListTile(
                  leading: space.id == selection.spaceId
                      ? const Icon(Icons.check)
                      : null,
                  title: Text(space.name),
                  subtitle: Text(space.slug),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () {
                    ref
                        .read(spaceSelectionProvider.notifier)
                        .selectSpace(spaceId: space.id, spaceName: space.name);
                    GoRouter.of(context).go(AppRoutes.chat);
                  },
                ),
              ),
          ],
        );
      },
    );
  }
}

class _ErrorCard extends StatelessWidget {
  const _ErrorCard({required this.message, this.onRetry});
  final String message;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Theme.of(context).colorScheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Text(
              message,
              style: TextStyle(
                color: Theme.of(context).colorScheme.onErrorContainer,
              ),
            ),
            if (onRetry != null) ...[
              const SizedBox(height: 12),
              FilledButton.tonal(
                onPressed: onRetry,
                child: const Text('Retry'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _EmptyCard extends StatelessWidget {
  const _EmptyCard({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          message,
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.bodyLarge,
        ),
      ),
    );
  }
}
