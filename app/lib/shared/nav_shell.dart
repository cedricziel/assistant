import 'package:cupertino_sidebar/cupertino_sidebar.dart';
import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../api/connectivity_provider.dart';
import '../features/chat/chat_provider.dart';
import '../features/contexts/providers/context_providers.dart';
import '../features/notifications/agent_event_listener.dart';
import '../features/pwa/pwa_provider.dart';
import '../features/spaces/space_provider.dart';
import '../features/updater/update_banner.dart';
import '../router/app_router.dart';
import 'platform/platform.dart';
import 'space_switcher.dart';

/// Breakpoint above which the navigation rail is shown instead of bottom nav.
const double _kNavRailBreakpoint = 768;

/// Sidebar widths for the Material collapsible sidebar.
const double _kSidebarExpandedWidth = 240.0;
const double _kSidebarCollapsedWidth = 72.0;
const Duration _kSidebarAnimDuration = Duration(milliseconds: 200);

/// Whether the navigation sidebar is collapsed to icon-only mode on wide screens.
class SidebarCollapsedNotifier extends Notifier<bool> {
  @override
  bool build() => false;

  void toggle() => state = !state;
}

final sidebarCollapsedProvider =
    NotifierProvider<SidebarCollapsedNotifier, bool>(
      SidebarCollapsedNotifier.new,
    );

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

// Context switcher — exposed via trailing slot on desktop, More sheet on mobile.
const _NavDest _contextsDestination = _NavDest(
  path: '/contexts',
  icon: Icons.swap_horiz_outlined,
  selectedIcon: Icons.swap_horiz,
  label: 'Contexts',
);

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
    path: '/admin',
    icon: Icons.admin_panel_settings_outlined,
    selectedIcon: Icons.admin_panel_settings,
    label: 'Admin',
  ),
];

// Settings + Contexts switcher — sticky trailing slot on desktop, More sheet on mobile.
const _NavDest _settingsDestination = _NavDest(
  path: '/settings',
  icon: Icons.settings_outlined,
  selectedIcon: Icons.settings,
  label: 'Settings',
);

/// Tab bar destinations used on iOS: primary + Contexts promoted to a tab.
final List<_NavDest> _iosTabDestinations = [
  _primaryDestinations[0], // Chat
  _contextsDestination, // Contexts (promoted from overflow)
  _primaryDestinations[1], // Skills
  _primaryDestinations[2], // Workflows
];

/// Flat list of all destinations shown in the Apple sidebar, in the order
/// CupertinoSidebar assigns indices: primary, then overflow (inside the
/// SidebarSection), then settings.
final List<_NavDest> _appleSidebarDestinations = [
  ..._iosTabDestinations,
  ..._overflowDestinations,
  _settingsDestination,
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
class NavShell extends ConsumerStatefulWidget {
  const NavShell({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<NavShell> createState() => _NavShellState();
}

class _NavShellState extends ConsumerState<NavShell> {
  bool _creatingChat = false;

  Future<void> _createNewChat() async {
    if (_creatingChat) return;
    _creatingChat = true;
    try {
      final id = await ref
          .read(conversationListProvider.notifier)
          .createConversation();
      if (id != null && mounted) {
        context.go('/chat/$id');
      }
    } finally {
      _creatingChat = false;
    }
  }

  /// Index for the mobile [NavigationBar].
  ///
  /// Primary destinations occupy indices 0–3; index 4 is the "More" item.
  /// When the current route is an overflow destination "More" is active (4).
  int _mobileSelectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _primaryDestinations.length; i++) {
      if (location.startsWith(_primaryDestinations[i].path)) return i;
    }
    // Contexts, Settings, and all overflow destinations map to "More".
    if (location.startsWith(_contextsDestination.path) ||
        location.startsWith(_settingsDestination.path) ||
        _isOverflowRouteActive(location)) {
      return _primaryDestinations.length;
    }
    return 0;
  }

  /// Index for the Apple [CupertinoSidebar].
  ///
  /// Maps the current route to the flat [_appleSidebarDestinations] list.
  int _appleSidebarSelectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _appleSidebarDestinations.length; i++) {
      if (location.startsWith(_appleSidebarDestinations[i].path)) return i;
    }
    return 0;
  }

  /// Index for the Material collapsible sidebar.
  ///
  /// Maps to the flat list: primary destinations then overflow destinations.
  /// Settings and Contexts are in the trailing section, not indexed.
  int _materialSidebarSelectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _primaryDestinations.length; i++) {
      if (location.startsWith(_primaryDestinations[i].path)) return i;
    }
    for (var i = 0; i < _overflowDestinations.length; i++) {
      if (location.startsWith(_overflowDestinations[i].path)) {
        return _primaryDestinations.length + i;
      }
    }
    return -1; // No match — settings/contexts are in trailing.
  }

  /// Shows the "More" bottom sheet listing all overflow destinations.
  void _showMoreSheet(BuildContext context) {
    showModalBottomSheet<void>(
      context: context,
      builder: (sheetContext) {
        return Semantics(
          label: 'More destinations',
          child: SafeArea(
            child: SingleChildScrollView(
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
                  // Space switcher at the top of the More sheet.
                  ListTile(
                    leading: const Icon(Icons.workspaces_outlined),
                    title: const Text('Switch space'),
                    onTap: () {
                      Navigator.of(sheetContext).pop();
                      context.go(AppRoutes.spaceSelector);
                    },
                  ),
                  // Contexts switcher.
                  ListTile(
                    leading: Icon(_contextsDestination.icon),
                    title: Text(_contextsDestination.label),
                    onTap: () {
                      Navigator.of(sheetContext).pop();
                      context.go(_contextsDestination.path);
                    },
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
                  // Settings is separated from the rest of the overflow items.
                  const Divider(height: 1),
                  ListTile(
                    leading: Icon(_settingsDestination.icon),
                    title: Text(_settingsDestination.label),
                    onTap: () {
                      Navigator.of(sheetContext).pop();
                      context.go(_settingsDestination.path);
                    },
                  ),
                  const SizedBox(height: 8),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  /// Index for the iOS [CupertinoTabBar].
  ///
  /// iOS tab destinations: Chat(0), Contexts(1), Skills(2), Workflows(3).
  /// Index 4 is the "More" item for overflow destinations.
  int _iosMobileSelectedIndex(BuildContext context) {
    final location = GoRouterState.of(context).uri.toString();
    for (var i = 0; i < _iosTabDestinations.length; i++) {
      if (location.startsWith(_iosTabDestinations[i].path)) return i;
    }
    // Settings and all overflow destinations map to "More".
    if (location.startsWith(_settingsDestination.path) ||
        _isOverflowRouteActive(location)) {
      return _iosTabDestinations.length;
    }
    return 0;
  }

  /// Shows a Cupertino action sheet listing all overflow destinations.
  void _showCupertinoMoreSheet(BuildContext context) {
    showCupertinoModalPopup<void>(
      context: context,
      builder: (popupContext) {
        return CupertinoActionSheet(
          title: const Text('More'),
          actions: [
            CupertinoActionSheetAction(
              onPressed: () {
                Navigator.of(popupContext).pop();
                context.go(AppRoutes.spaceSelector);
              },
              child: const Text('Switch space'),
            ),
            ..._overflowDestinations.map(
              (d) => CupertinoActionSheetAction(
                onPressed: () {
                  Navigator.of(popupContext).pop();
                  context.go(d.path);
                },
                child: Text(d.label),
              ),
            ),
            CupertinoActionSheetAction(
              onPressed: () {
                Navigator.of(popupContext).pop();
                context.go(_settingsDestination.path);
              },
              child: Text(_settingsDestination.label),
            ),
          ],
          cancelButton: CupertinoActionSheetAction(
            isDefaultAction: true,
            onPressed: () => Navigator.of(popupContext).pop(),
            child: const Text('Cancel'),
          ),
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    final child = widget.child;
    final width = MediaQuery.of(context).size.width;
    final isWide = width >= _kNavRailBreakpoint;

    // Watch install-prompt availability (false on non-web platforms).
    final isInstallable = ref.watch(pwaInstallProvider);
    // True when on iOS Safari and not yet installed as a PWA.
    final isSafariInstallable = ref.watch(safariInstallProvider);

    // Platform-aware Cmd+N / Ctrl+N shortcut for creating a new chat.
    final useMeta =
        defaultTargetPlatform == TargetPlatform.macOS ||
        defaultTargetPlatform == TargetPlatform.iOS;
    final newChatShortcuts = CallbackShortcuts(
      bindings: {
        SingleActivator(
          LogicalKeyboardKey.keyN,
          meta: useMeta,
          control: !useMeta,
        ): _createNewChat,
      },
      child: Focus(
        autofocus: true,
        child: _buildBody(
          context,
          child,
          isWide,
          isInstallable,
          isSafariInstallable,
        ),
      ),
    );

    return newChatShortcuts;
  }

  Widget _buildBody(
    BuildContext context,
    Widget child,
    bool isWide,
    bool isInstallable,
    bool isSafariInstallable,
  ) {
    // Apple touch: CupertinoTabBar (compact) or sidebar (wide).
    if (isAppleTouch) {
      if (isWide) {
        final collapsed = ref.watch(sidebarCollapsedProvider);
        final appleSelected = _appleSidebarSelectedIndex(context);
        return Scaffold(
          body: SafeArea(
            child: Stack(
              children: [
                Row(
                  children: [
                    CupertinoSidebarCollapsible(
                      isExpanded: !collapsed,
                      child: CupertinoSidebar(
                        selectedIndex: appleSelected,
                        navigationBar: _CupertinoSpaceSwitcher(),
                        onDestinationSelected: (i) {
                          final dest = _appleSidebarDestinations[i];
                          context.go(dest.path);
                        },
                        children: [
                          // Primary destinations
                          for (final d in _iosTabDestinations)
                            SidebarDestination(
                              icon: Icon(d.icon),
                              label: Text(d.label),
                            ),
                          // Overflow section
                          SidebarSection(
                            label: const Text('More'),
                            children: [
                              for (final d in _overflowDestinations)
                                SidebarDestination(
                                  icon: Icon(d.icon),
                                  label: Text(d.label),
                                ),
                            ],
                          ),
                          // Settings
                          SidebarDestination(
                            icon: Icon(_settingsDestination.icon),
                            label: Text(_settingsDestination.label),
                          ),
                        ],
                      ),
                    ),
                    Expanded(
                      child: AgentEventListener(
                        child: _OfflineBanner(
                          child: UpdateBannerWrapper(child: child),
                        ),
                      ),
                    ),
                  ],
                ),
                // Toggle button overlaid at top-leading corner.
                SafeArea(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    child: CupertinoButton(
                      padding: EdgeInsets.zero,
                      onPressed: () =>
                          ref.read(sidebarCollapsedProvider.notifier).toggle(),
                      child: Icon(
                        CupertinoIcons.sidebar_left,
                        semanticLabel: collapsed
                            ? 'Expand sidebar'
                            : 'Collapse sidebar',
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      }

      // iOS compact: CupertinoTabBar with 5 items.
      final selected = _iosMobileSelectedIndex(context);
      return Scaffold(
        body: AgentEventListener(
          child: _OfflineBanner(child: UpdateBannerWrapper(child: child)),
        ),
        bottomNavigationBar: CupertinoTabBar(
          currentIndex: selected,
          onTap: (i) {
            if (i == _iosTabDestinations.length) {
              _showCupertinoMoreSheet(context);
            } else {
              context.go(_iosTabDestinations[i].path);
            }
          },
          items: [
            for (final d in _iosTabDestinations)
              BottomNavigationBarItem(
                icon: Icon(d.icon),
                activeIcon: Icon(d.selectedIcon),
                label: d.label,
              ),
            const BottomNavigationBarItem(
              icon: Icon(Icons.more_horiz),
              activeIcon: Icon(Icons.more_horiz),
              label: 'More',
            ),
          ],
        ),
      );
    }

    if (isWide) {
      final collapsed = ref.watch(sidebarCollapsedProvider);
      final location = GoRouterState.of(context).uri.toString();
      final selected = _materialSidebarSelectedIndex(context);

      return Scaffold(
        body: SafeArea(
          child: Row(
            children: [
              // Collapsible sidebar: expanded shows icon + label list tiles,
              // collapsed shows a narrow icon-only rail.
              AnimatedContainer(
                duration: _kSidebarAnimDuration,
                curve: Curves.easeInOut,
                width: collapsed
                    ? _kSidebarCollapsedWidth
                    : _kSidebarExpandedWidth,
                clipBehavior: Clip.hardEdge,
                decoration: const BoxDecoration(),
                // OverflowBox bypasses parent constraints so ListTile
                // always receives the target width, not an intermediate
                // animation value. The AnimatedContainer clips the overflow.
                child: OverflowBox(
                  maxWidth: collapsed
                      ? _kSidebarCollapsedWidth
                      : _kSidebarExpandedWidth,
                  minWidth: collapsed
                      ? _kSidebarCollapsedWidth
                      : _kSidebarExpandedWidth,
                  alignment: Alignment.topCenter,
                  child: Column(
                    children: [
                      // Toggle button row
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 4),
                        child: Align(
                          alignment: collapsed
                              ? Alignment.center
                              : Alignment.centerRight,
                          child: IconButton(
                            icon: Icon(
                              collapsed ? Icons.menu : Icons.menu_open,
                            ),
                            tooltip: collapsed
                                ? 'Expand sidebar'
                                : 'Collapse sidebar',
                            onPressed: () => ref
                                .read(sidebarCollapsedProvider.notifier)
                                .toggle(),
                          ),
                        ),
                      ),
                      // Scrollable navigation items
                      Expanded(
                        child: SingleChildScrollView(
                          child: Column(
                            children: [
                              for (
                                var i = 0;
                                i < _primaryDestinations.length;
                                i++
                              )
                                _MaterialSidebarItem(
                                  icon: _primaryDestinations[i].icon,
                                  selectedIcon:
                                      _primaryDestinations[i].selectedIcon,
                                  label: _primaryDestinations[i].label,
                                  selected: selected == i,
                                  collapsed: collapsed,
                                  onTap: () =>
                                      context.go(_primaryDestinations[i].path),
                                ),
                              Padding(
                                padding: collapsed
                                    ? const EdgeInsets.symmetric(
                                        horizontal: 12,
                                        vertical: 4,
                                      )
                                    : const EdgeInsets.symmetric(
                                        horizontal: 16,
                                        vertical: 4,
                                      ),
                                child: const Divider(height: 1),
                              ),
                              for (
                                var i = 0;
                                i < _overflowDestinations.length;
                                i++
                              )
                                _MaterialSidebarItem(
                                  icon: _overflowDestinations[i].icon,
                                  selectedIcon:
                                      _overflowDestinations[i].selectedIcon,
                                  label: _overflowDestinations[i].label,
                                  selected:
                                      selected ==
                                      _primaryDestinations.length + i,
                                  collapsed: collapsed,
                                  onTap: () =>
                                      context.go(_overflowDestinations[i].path),
                                ),
                            ],
                          ),
                        ),
                      ),
                      // Sticky trailing section
                      const Divider(height: 1),
                      SpaceSwitcher(collapsed: collapsed),
                      if (!kIsWeb)
                        _MaterialSidebarItem(
                          icon: _contextsDestination.icon,
                          selectedIcon: _contextsDestination.selectedIcon,
                          label: _contextsDestination.label,
                          selected: location.startsWith(
                            _contextsDestination.path,
                          ),
                          collapsed: collapsed,
                          onTap: () => context.go(_contextsDestination.path),
                        ),
                      if (kIsWeb)
                        _LogoutButton(
                          onLogout: () async {
                            await ref
                                .read(activeContextProvider.notifier)
                                .deactivate();
                            if (context.mounted) {
                              context.go(AppRoutes.login);
                            }
                          },
                        ),
                      _MaterialSidebarItem(
                        icon: _settingsDestination.icon,
                        selectedIcon: _settingsDestination.selectedIcon,
                        label: _settingsDestination.label,
                        selected: location.startsWith(
                          _settingsDestination.path,
                        ),
                        collapsed: collapsed,
                        onTap: () => context.go(_settingsDestination.path),
                      ),
                      if (isInstallable)
                        _InstallButton(
                          onTap: () =>
                              ref.read(pwaInstallProvider.notifier).install(),
                        ),
                      if (isSafariInstallable) const _SafariInstallButton(),
                      const SizedBox(height: 8),
                    ],
                  ),
                ),
              ),
              const VerticalDivider(width: 1, thickness: 1),
              Expanded(
                child: AgentEventListener(
                  child: _OfflineBanner(
                    child: UpdateBannerWrapper(child: child),
                  ),
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
      body: AgentEventListener(
        child: _OfflineBanner(child: UpdateBannerWrapper(child: child)),
      ),
      bottomNavigationBar: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isInstallable)
            _InstallBanner(
              onInstall: () => ref.read(pwaInstallProvider.notifier).install(),
            ),
          if (isSafariInstallable) const _SafariInstallBanner(),
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

/// Material sidebar navigation item that adapts between expanded and collapsed
/// modes. Expanded shows icon + label in a rounded ListTile; collapsed shows a
/// centered icon with a pill indicator and a tooltip.
class _MaterialSidebarItem extends StatelessWidget {
  const _MaterialSidebarItem({
    required this.icon,
    required this.selectedIcon,
    required this.label,
    required this.selected,
    required this.collapsed,
    required this.onTap,
  });

  final IconData icon;
  final IconData selectedIcon;
  final String label;
  final bool selected;
  final bool collapsed;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    if (collapsed) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 2),
        child: Tooltip(
          message: label,
          child: InkWell(
            onTap: onTap,
            borderRadius: BorderRadius.circular(16),
            child: Center(
              child: Container(
                width: 56,
                height: 32,
                alignment: Alignment.center,
                decoration: selected
                    ? BoxDecoration(
                        color: colorScheme.secondaryContainer,
                        borderRadius: BorderRadius.circular(16),
                      )
                    : null,
                child: Icon(
                  selected ? selectedIcon : icon,
                  color: selected
                      ? colorScheme.onSecondaryContainer
                      : colorScheme.onSurfaceVariant,
                  size: 22,
                ),
              ),
            ),
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 1),
      child: ListTile(
        dense: true,
        leading: Icon(
          selected ? selectedIcon : icon,
          color: selected
              ? colorScheme.onSecondaryContainer
              : colorScheme.onSurfaceVariant,
          size: 22,
        ),
        title: Text(
          label,
          style: TextStyle(
            color: selected
                ? colorScheme.onSecondaryContainer
                : colorScheme.onSurface,
            fontWeight: selected ? FontWeight.w600 : FontWeight.normal,
          ),
          overflow: TextOverflow.ellipsis,
        ),
        selected: selected,
        selectedTileColor: colorScheme.secondaryContainer,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(28)),
        onTap: onTap,
      ),
    );
  }
}

/// Logout icon button shown in the [NavigationRail] trailing slot on web.
class _LogoutButton extends StatelessWidget {
  const _LogoutButton({required this.onLogout});

  final VoidCallback onLogout;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.logout),
      tooltip: 'Log out',
      onPressed: onLogout,
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

/// Icon button shown in the [NavigationRail] trailing slot on iOS Safari.
/// Tapping it opens a bottom sheet explaining how to add to home screen.
class _SafariInstallButton extends StatelessWidget {
  const _SafariInstallButton();

  @override
  Widget build(BuildContext context) {
    return IconButton.filledTonal(
      icon: const Icon(Icons.ios_share),
      tooltip: 'Add to Home Screen',
      onPressed: () => _showSafariInstructions(context),
    );
  }
}

/// Slim banner above the mobile [NavigationBar] on iOS Safari.
class _SafariInstallBanner extends StatelessWidget {
  const _SafariInstallBanner();

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.primaryContainer,
      child: InkWell(
        onTap: () => _showSafariInstructions(context),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Icon(Icons.ios_share, color: colorScheme.onPrimaryContainer),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  'Add to Home Screen',
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

/// Shows a bottom sheet explaining iOS Safari's manual install steps.
void _showSafariInstructions(BuildContext context) {
  showModalBottomSheet<void>(
    context: context,
    builder: (sheetContext) {
      final colorScheme = Theme.of(sheetContext).colorScheme;
      return SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(24, 24, 24, 32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Add to Home Screen',
                style: Theme.of(sheetContext).textTheme.titleLarge,
              ),
              const SizedBox(height: 16),
              _SafariStep(
                number: '1',
                icon: Icons.ios_share,
                color: colorScheme.primary,
                text: 'Tap the Share button in the Safari toolbar.',
              ),
              const SizedBox(height: 12),
              _SafariStep(
                number: '2',
                icon: Icons.add_box_outlined,
                color: colorScheme.primary,
                text: 'Scroll down and tap "Add to Home Screen".',
              ),
              const SizedBox(height: 12),
              _SafariStep(
                number: '3',
                icon: Icons.check_circle_outline,
                color: colorScheme.primary,
                text: 'Tap "Add" to confirm.',
              ),
            ],
          ),
        ),
      );
    },
  );
}

class _SafariStep extends StatelessWidget {
  const _SafariStep({
    required this.number,
    required this.icon,
    required this.color,
    required this.text,
  });

  final String number;
  final IconData icon;
  final Color color;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        CircleAvatar(
          radius: 14,
          backgroundColor: color,
          child: Text(
            number,
            style: TextStyle(
              color: Theme.of(context).colorScheme.onPrimary,
              fontWeight: FontWeight.bold,
              fontSize: 13,
            ),
          ),
        ),
        const SizedBox(width: 12),
        Icon(icon, color: color),
        const SizedBox(width: 8),
        Expanded(child: Text(text)),
      ],
    );
  }
}

/// Cupertino-styled space switcher used as the [CupertinoSidebar.navigationBar].
///
/// Wraps [SidebarNavigationBar] (which renders a [CupertinoSliverNavigationBar])
/// so it integrates as a proper sliver inside the sidebar's [CustomScrollView].
class _CupertinoSpaceSwitcher extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selection = ref.watch(spaceSelectionProvider);
    final spaceName = selection.spaceName;

    return SidebarNavigationBar(
      title: Text(spaceName ?? 'Spaces'),
      trailing: CupertinoButton(
        padding: EdgeInsets.zero,
        onPressed: () => context.go(AppRoutes.spaceSelector),
        child: const Icon(CupertinoIcons.arrow_2_squarepath, size: 20),
      ),
    );
  }
}

/// Persistent banner shown when the device has no network connectivity.
///
/// Skipped on web — browsers provide their own offline indicators.
/// Uses a Cupertino-style slim bar on iOS, MaterialBanner elsewhere.
class _OfflineBanner extends ConsumerWidget {
  const _OfflineBanner({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (kIsWeb) return child;

    final isOnline = ref.watch(isOnlineProvider);
    if (isOnline) return child;

    return Column(
      children: [
        if (isAppleTouch)
          Builder(
            builder: (context) {
              final fgColor = Theme.of(context).colorScheme.onInverseSurface;
              return Container(
                width: double.infinity,
                color: CupertinoColors.systemOrange.resolveFrom(context),
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(CupertinoIcons.wifi_slash, size: 14, color: fgColor),
                    const SizedBox(width: 6),
                    Text(
                      'No Connection',
                      style: TextStyle(
                        color: fgColor,
                        fontSize: 13,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                  ],
                ),
              );
            },
          )
        else
          MaterialBanner(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            leading: const Icon(Icons.wifi_off),
            content: const Text('No internet connection'),
            actions: const [SizedBox.shrink()],
          ),
        Expanded(child: child),
      ],
    );
  }
}
