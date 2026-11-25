import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../features/pwa/pwa_provider.dart';
import '../edge_swipe_sidebar_toggle.dart';
import '../sidebar_toggle_button.dart';
import '../space_switcher.dart';
import 'install_prompts.dart';
import 'nav_destinations.dart';
import 'nav_logout.dart';
import 'shell_content.dart';
import 'sidebar_collapsed_provider.dart';

/// Sidebar widths for the Material collapsible sidebar.
const double _kSidebarExpandedWidth = 240.0;
const double _kSidebarCollapsedWidth = 72.0;
const Duration _kSidebarAnimDuration = Duration(milliseconds: 200);

/// Wide-screen Material layout: a collapsible sidebar beside the routed page.
class MaterialWideLayout extends ConsumerWidget {
  const MaterialWideLayout({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isInstallable = ref.watch(pwaInstallProvider);
    final isSafariInstallable = ref.watch(safariInstallProvider);
    final collapsed = ref.watch(sidebarCollapsedProvider).value ?? false;
    final location = GoRouterState.of(context).uri.toString();
    final selected = materialSidebarSelectedIndex(location);

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
                          icon: Icon(collapsed ? Icons.menu : Icons.menu_open),
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
                            for (var i = 0; i < primaryDestinations.length; i++)
                              _MaterialSidebarItem(
                                icon: primaryDestinations[i].icon,
                                selectedIcon:
                                    primaryDestinations[i].selectedIcon,
                                label: primaryDestinations[i].label,
                                selected: selected == i,
                                collapsed: collapsed,
                                onTap: () =>
                                    context.go(primaryDestinations[i].path),
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
                              i < overflowDestinations.length;
                              i++
                            )
                              _MaterialSidebarItem(
                                icon: overflowDestinations[i].icon,
                                selectedIcon:
                                    overflowDestinations[i].selectedIcon,
                                label: overflowDestinations[i].label,
                                selected:
                                    selected == primaryDestinations.length + i,
                                collapsed: collapsed,
                                onTap: () =>
                                    context.go(overflowDestinations[i].path),
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
                        icon: contextsDestination.icon,
                        selectedIcon: contextsDestination.selectedIcon,
                        label: contextsDestination.label,
                        selected: location.startsWith(contextsDestination.path),
                        collapsed: collapsed,
                        onTap: () => context.go(contextsDestination.path),
                      ),
                    _LogoutButton(
                      onLogout: () async {
                        await logoutAndShowLogin(context);
                      },
                    ),
                    _MaterialSidebarItem(
                      icon: settingsDestination.icon,
                      selectedIcon: settingsDestination.selectedIcon,
                      label: settingsDestination.label,
                      selected: location.startsWith(settingsDestination.path),
                      collapsed: collapsed,
                      onTap: () => context.go(settingsDestination.path),
                    ),
                    if (isInstallable)
                      InstallButton(
                        onTap: () =>
                            ref.read(pwaInstallProvider.notifier).install(),
                      ),
                    if (isSafariInstallable) const SafariInstallButton(),
                    const SizedBox(height: 8),
                  ],
                ),
              ),
            ),
            const VerticalDivider(width: 1, thickness: 1),
            Expanded(
              child: EdgeSwipeSidebarToggle(
                child: Stack(
                  children: [
                    ShellContent(child: child),
                    // Top-leading toggle overlay so the collapse affordance
                    // is visible outside the sidebar rail (iPad landscape /
                    // web tablet users).
                    Positioned(
                      top: 4,
                      left: 4,
                      child: SafeArea(child: const SidebarToggleButton()),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
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
