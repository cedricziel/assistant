import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/spaces/space_provider.dart';
import '../router/app_router.dart';

/// Compact widget that displays the active org/space and navigates to the
/// space selector screen on tap.
///
/// When [collapsed] is true (icon-only sidebar), renders a single icon button
/// with a tooltip showing the current org/space. When expanded, renders an
/// [InkWell] with the org name as the title and the space name as subtitle.
class SpaceSwitcher extends ConsumerWidget {
  const SpaceSwitcher({super.key, required this.collapsed});

  final bool collapsed;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selection = ref.watch(spaceSelectionProvider);

    final orgName = selection.orgName;
    final spaceName = selection.spaceName;
    final hasOrg = orgName != null;
    final hasSpace = spaceName != null;

    final tooltipText = hasOrg && hasSpace
        ? '$orgName / $spaceName'
        : hasOrg
        ? orgName
        : 'Select space';

    if (collapsed) {
      return Tooltip(
        message: tooltipText,
        child: InkWell(
          onTap: () => context.go(AppRoutes.spaceSelector),
          borderRadius: BorderRadius.circular(16),
          child: const Center(
            child: SizedBox(
              width: 56,
              height: 32,
              child: Icon(Icons.workspaces_outlined, size: 22),
            ),
          ),
        ),
      );
    }

    final colorScheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      child: InkWell(
        onTap: () => context.go(AppRoutes.spaceSelector),
        borderRadius: BorderRadius.circular(28),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Row(
            children: [
              Icon(
                Icons.workspaces_outlined,
                size: 20,
                color: colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      hasOrg ? orgName : 'Select space',
                      style: TextStyle(
                        fontSize: 13,
                        fontWeight: FontWeight.w600,
                        color: colorScheme.onSurface,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                    if (hasOrg)
                      Text(
                        hasSpace ? spaceName : 'Select space',
                        style: TextStyle(
                          fontSize: 11,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                  ],
                ),
              ),
              Icon(
                Icons.unfold_more,
                size: 16,
                color: colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
