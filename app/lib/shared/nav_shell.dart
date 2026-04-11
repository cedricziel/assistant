import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../features/notifications/agent_event_listener.dart';
import '../features/pwa/pwa_provider.dart';
import '../features/updater/update_banner.dart';
import 'server_chip.dart';

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

const List<_NavDest> _primaryDestinations = [
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
];

const _NavDest _settingsDest = _NavDest(
  path: '/settings',
  icon: Icons.settings_outlined,
  selectedIcon: Icons.settings,
  label: 'Settings',
);

const List<_NavDest> _destinations = [..._primaryDestinations, _settingsDest];

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
              SingleChildScrollView(
                child: ConstrainedBox(
                  constraints: BoxConstraints(
                    minHeight:
                        MediaQuery.of(context).size.height -
                        MediaQuery.of(context).padding.top -
                        MediaQuery.of(context).padding.bottom,
                  ),
                  child: IntrinsicHeight(
                    child: NavigationRail(
                      selectedIndex: selected < _primaryDestinations.length
                          ? selected
                          : 0,
                      labelType: NavigationRailLabelType.all,
                      destinations: _primaryDestinations
                          .map(
                            (d) => NavigationRailDestination(
                              icon: Icon(d.icon),
                              selectedIcon: Icon(d.selectedIcon),
                              label: Text(d.label),
                            ),
                          )
                          .toList(),
                      onDestinationSelected: (i) {
                        context.go(_primaryDestinations[i].path);
                      },
                      trailing: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            icon: Icon(
                              selected == _destinations.length - 1
                                  ? Icons.settings
                                  : Icons.settings_outlined,
                            ),
                            tooltip: 'Settings',
                            onPressed: () => context.go('/settings'),
                          ),
                          const SizedBox(height: 4),
                          const ServerChip(),
                          const SizedBox(height: 4),
                          if (isInstallable)
                            Padding(
                              padding: const EdgeInsets.symmetric(vertical: 8),
                              child: _InstallButton(
                                onTap: () => ref
                                    .read(pwaInstallProvider.notifier)
                                    .install(),
                              ),
                            ),
                          const SizedBox(height: 8),
                        ],
                      ),
                    ),
                  ),
                ),
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

    // Mobile layout: omnipresent drawer replaces the bottom navigation bar.
    // All destinations are reachable from the drawer; Settings is separated
    // by a Divider. The PWA install option appears at the bottom when available.
    return Scaffold(
      appBar: AppBar(
        leading: const DrawerButton(),
        title: const Text('Assistant'),
      ),
      drawer: Drawer(
        child: ListView(
          padding: EdgeInsets.zero,
          children: [
            const DrawerHeader(padding: EdgeInsets.zero, child: ServerChip()),
            for (var i = 0; i < _primaryDestinations.length; i++)
              ListTile(
                leading: Icon(
                  selected == i
                      ? _primaryDestinations[i].selectedIcon
                      : _primaryDestinations[i].icon,
                ),
                title: Text(_primaryDestinations[i].label),
                selected: selected == i,
                onTap: () {
                  context.go(_primaryDestinations[i].path);
                  Navigator.of(context).pop();
                },
              ),
            const Divider(),
            ListTile(
              leading: Icon(
                selected == _destinations.length - 1
                    ? _settingsDest.selectedIcon
                    : _settingsDest.icon,
              ),
              title: Text(_settingsDest.label),
              selected: selected == _destinations.length - 1,
              onTap: () {
                context.go(_settingsDest.path);
                Navigator.of(context).pop();
              },
            ),
            if (isInstallable)
              ListTile(
                leading: const Icon(Icons.install_mobile),
                title: const Text('Install App'),
                onTap: () {
                  ref.read(pwaInstallProvider.notifier).install();
                  Navigator.of(context).pop();
                },
              ),
          ],
        ),
      ),
      body: AgentEventListener(child: UpdateBannerWrapper(child: child)),
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
