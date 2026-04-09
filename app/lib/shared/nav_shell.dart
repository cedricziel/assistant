import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/notifications/agent_event_listener.dart';
import '../features/pwa/pwa_provider.dart';
import '../features/updater/update_banner.dart';

/// Breakpoint above which the navigation rail is shown instead of bottom nav.
const double _kNavRailBreakpoint = 768;

/// Navigation destination model.
class _NavDest {
  const _NavDest({
    required this.path,
    required this.icon,
    required this.selectedIcon,
    required this.label,
  });

  final String path;
  final IconData icon;
  final IconData selectedIcon;
  final String label;
}

const List<_NavDest> _destinations = [
  _NavDest(
    path: '/chat',
    icon: Icons.chat_bubble_outline,
    selectedIcon: Icons.chat_bubble,
    label: 'Chat',
  ),
  _NavDest(
    path: '/traces',
    icon: Icons.timeline_outlined,
    selectedIcon: Icons.timeline,
    label: 'Traces',
  ),
  _NavDest(
    path: '/logs',
    icon: Icons.article_outlined,
    selectedIcon: Icons.article,
    label: 'Logs',
  ),
  _NavDest(
    path: '/contexts',
    icon: Icons.person_outline,
    selectedIcon: Icons.person,
    label: 'Personas',
  ),
  _NavDest(
    path: '/skills',
    icon: Icons.extension_outlined,
    selectedIcon: Icons.extension,
    label: 'Skills',
  ),
  _NavDest(
    path: '/workflows',
    icon: Icons.account_tree_outlined,
    selectedIcon: Icons.account_tree,
    label: 'Workflows',
  ),
  _NavDest(
    path: '/webhooks',
    icon: Icons.webhook_outlined,
    selectedIcon: Icons.webhook,
    label: 'Webhooks',
  ),
  _NavDest(
    path: '/agents',
    icon: Icons.smart_toy_outlined,
    selectedIcon: Icons.smart_toy,
    label: 'Agents',
  ),
  _NavDest(
    path: '/analytics',
    icon: Icons.bar_chart_outlined,
    selectedIcon: Icons.bar_chart,
    label: 'Analytics',
  ),
  _NavDest(
    path: '/settings',
    icon: Icons.settings_outlined,
    selectedIcon: Icons.settings,
    label: 'Settings',
  ),
];

/// Shell widget that wraps all main screens with persistent navigation.
///
/// Uses a [NavigationRail] on tablet/desktop (>= 768px wide) and a
/// [BottomNavigationBar] on mobile (< 768px wide).
///
/// When the PWA install prompt is available (web only), an "Install App"
/// button is shown: in the [NavigationRail] trailing area on wide screens,
/// and as a banner above the [NavigationBar] on narrow screens.
class NavShell extends ConsumerWidget {
  const NavShell({super.key, required this.child});

  final Widget child;

  int _selectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _destinations.length; i++) {
      if (location.startsWith(_destinations[i].path)) {
        return i;
      }
    }
    return 0;
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final width = MediaQuery.of(context).size.width;
    final isWide = width >= _kNavRailBreakpoint;
    final selected = _selectedIndex(context);

    // Watch install-prompt availability (false on non-web platforms).
    final isInstallable = ref.watch(pwaInstallProvider);

    if (isWide) {
      return Scaffold(
        body: SafeArea(
          child: Row(
            children: [
              NavigationRail(
                selectedIndex: selected,
                labelType: NavigationRailLabelType.all,
                destinations: _destinations
                    .map(
                      (d) => NavigationRailDestination(
                        icon: Icon(d.icon),
                        selectedIcon: Icon(d.selectedIcon),
                        label: Text(d.label),
                      ),
                    )
                    .toList(),
                onDestinationSelected: (i) {
                  context.go(_destinations[i].path);
                },
                // Show the install button at the bottom of the rail when the
                // browser's install prompt is available.
                trailing: isInstallable
                    ? Padding(
                        padding: const EdgeInsets.symmetric(vertical: 8),
                        child: _InstallButton(
                          onTap: () =>
                              ref.read(pwaInstallProvider.notifier).install(),
                        ),
                      )
                    : null,
              ),
              const VerticalDivider(width: 1, thickness: 1),
              Expanded(
        child: AgentEventListener(
          child: UpdateBannerWrapper(child: child),
        ),
      ),
            ],
          ),
        ),
      );
    }

    // Mobile layout: show an install-app banner above the NavigationBar when
    // the install prompt is available.
    return Scaffold(
      body: AgentEventListener(child: UpdateBannerWrapper(child: child)),
      bottomNavigationBar: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isInstallable)
            _InstallBanner(
              onInstall: () => ref.read(pwaInstallProvider.notifier).install(),
            ),
          NavigationBar(
            selectedIndex: selected,
            onDestinationSelected: (i) {
              context.go(_destinations[i].path);
            },
            destinations: _destinations
                .map(
                  (d) => NavigationDestination(
                    icon: Icon(d.icon),
                    selectedIcon: Icon(d.selectedIcon),
                    label: d.label,
                  ),
                )
                .toList(),
          ),
        ],
      ),
    );
  }
}

/// Compact icon button used in the [NavigationRail] trailing slot.
class _InstallButton extends StatelessWidget {
  const _InstallButton({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return IconButton.filledTonal(
      icon: const Icon(Icons.install_mobile),
      tooltip: 'Install App',
      onPressed: onTap,
    );
  }
}

/// Slim banner shown above the mobile [NavigationBar].
class _InstallBanner extends StatelessWidget {
  const _InstallBanner({required this.onInstall});

  final VoidCallback onInstall;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.primaryContainer,
      child: InkWell(
        onTap: onInstall,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Icon(Icons.install_mobile, color: colorScheme.onPrimaryContainer),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  'Install App',
                  style: TextStyle(
                    color: colorScheme.onPrimaryContainer,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
              Icon(Icons.arrow_forward, color: colorScheme.onPrimaryContainer),
            ],
          ),
        ),
      ),
    );
  }
}
