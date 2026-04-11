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

// -- Primary destinations (always visible on mobile bottom bar) --

const List<_NavDest> _primaryDestinations = [
  _NavDest(
    path: '/chat',
    icon: Icons.chat_bubble_outline,
    selectedIcon: Icons.chat_bubble,
    label: 'Chat',
  ),
  _NavDest(
    path: '/contexts',
    icon: Icons.swap_horiz_outlined,
    selectedIcon: Icons.swap_horiz,
    label: 'Contexts',
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
];

// -- Overflow destinations (in "More" sheet on mobile; below divider on desktop rail) --

const List<_NavDest> _overflowDestinations = [
  _NavDest(
    path: '/personas',
    icon: Icons.person_outline,
    selectedIcon: Icons.person,
    label: 'Personas',
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

/// Returns true when [currentPath] belongs to any overflow destination.
bool _isOverflowRouteActive(String currentPath) {
  return _overflowDestinations.any((d) => currentPath.startsWith(d.path));
}

/// Shell widget that wraps all main screens with persistent navigation.
///
/// Uses a [NavigationRail] on tablet/desktop (>= 768px wide) and a
/// [NavigationBar] on mobile (< 768px wide).
///
/// **Mobile**: Shows 4 primary destinations + a "More" item that opens a
/// [ModalBottomSheet] listing overflow/developer destinations.
///
/// **Desktop**: Shows all destinations in a single [NavigationRail] with a
/// visual divider between primary and overflow groups.
///
/// When the PWA install prompt is available (web only), an "Install App"
/// button is shown: in the [NavigationRail] trailing area on wide screens,
/// and as a banner above the [NavigationBar] on narrow screens.
class NavShell extends ConsumerWidget {
  const NavShell({super.key, required this.child});

  final Widget child;

  /// Index for the mobile [NavigationBar].
  ///
  /// Primary destinations occupy indices 0–3; index 4 is the "More" item.
  /// When the current route is an overflow destination "More" is active (4).
  int _mobileSelectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _primaryDestinations.length; i++) {
      if (location.startsWith(_primaryDestinations[i].path)) return i;
    }
    if (_isOverflowRouteActive(location)) return _primaryDestinations.length;
    return 0;
  }

  /// Index for the desktop [NavigationRail].
  ///
  /// Primary destinations: 0–3. Divider sentinel: 4 (disabled, not selectable).
  /// Overflow destinations: 5–9.
  int _railSelectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _primaryDestinations.length; i++) {
      if (location.startsWith(_primaryDestinations[i].path)) return i;
    }
    for (var i = 0; i < _overflowDestinations.length; i++) {
      if (location.startsWith(_overflowDestinations[i].path)) {
        // +1 to skip the divider sentinel at index _primaryDestinations.length
        return _primaryDestinations.length + 1 + i;
      }
    }
    return 0;
  }

  /// Shows the "More" bottom sheet listing all overflow destinations.
  void _showMoreSheet(BuildContext context) {
    showModalBottomSheet<void>(
      context: context,
      builder: (sheetContext) {
        return Semantics(
          label: 'More destinations',
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
                child: Text(
                  'More',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              ..._overflowDestinations.map(
                (d) => ListTile(
                  leading: Icon(d.icon),
                  title: Text(d.label),
                  onTap: () {
                    Navigator.of(sheetContext).pop();
                    context.go(d.path);
                  },
                ),
              ),
              const SizedBox(height: 8),
            ],
          ),
        );
      },
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final width = MediaQuery.of(context).size.width;
    final isWide = width >= _kNavRailBreakpoint;

    // Watch install-prompt availability (false on non-web platforms).
    final isInstallable = ref.watch(pwaInstallProvider);

    if (isWide) {
      final selected = _railSelectedIndex(context);
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
                      selectedIndex: selected,
                      labelType: NavigationRailLabelType.all,
                      destinations: [
                        // Primary destinations (0–3)
                        ..._primaryDestinations.map(
                          (d) => NavigationRailDestination(
                            icon: Icon(d.icon),
                            selectedIcon: Icon(d.selectedIcon),
                            label: Text(d.label),
                          ),
                        ),
                        // Divider sentinel (index 4) — non-interactive
                        const NavigationRailDestination(
                          disabled: true,
                          icon: Padding(
                            padding: EdgeInsets.symmetric(vertical: 4),
                            child: Divider(thickness: 1),
                          ),
                          label: SizedBox.shrink(),
                        ),
                        // Overflow / developer destinations (5–9)
                        ..._overflowDestinations.map(
                          (d) => NavigationRailDestination(
                            icon: Icon(d.icon),
                            selectedIcon: Icon(d.selectedIcon),
                            label: Text(d.label),
                          ),
                        ),
                      ],
                      onDestinationSelected: (i) {
                        if (i < _primaryDestinations.length) {
                          context.go(_primaryDestinations[i].path);
                        } else if (i > _primaryDestinations.length) {
                          // Skip divider at index _primaryDestinations.length
                          final overflowIndex =
                              i - _primaryDestinations.length - 1;
                          context.go(_overflowDestinations[overflowIndex].path);
                        }
                        // i == _primaryDestinations.length is the divider — ignore
                      },
                      // Show the install button at the bottom of the rail when the
                      // browser's install prompt is available.
                      trailing: isInstallable
                          ? Padding(
                              padding: const EdgeInsets.symmetric(vertical: 8),
                              child: _InstallButton(
                                onTap: () => ref
                                    .read(pwaInstallProvider.notifier)
                                    .install(),
                              ),
                            )
                          : null,
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

    // Mobile layout: 4 primary destinations + "More" overflow item.
    // Show an install-app banner above the NavigationBar when the install
    // prompt is available.
    final selected = _mobileSelectedIndex(context);
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
              if (i == _primaryDestinations.length) {
                _showMoreSheet(context);
              } else {
                context.go(_primaryDestinations[i].path);
              }
            },
            destinations: [
              ..._primaryDestinations.map(
                (d) => NavigationDestination(
                  icon: Icon(d.icon),
                  selectedIcon: Icon(d.selectedIcon),
                  label: d.label,
                ),
              ),
              const NavigationDestination(
                icon: Icon(Icons.more_horiz_outlined),
                selectedIcon: Icon(Icons.more_horiz),
                label: 'More',
              ),
            ],
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
