import 'package:cupertino_sidebar/cupertino_sidebar.dart';
import 'package:cupertino_ui/cupertino_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../features/spaces/space_provider.dart';
import '../../router/app_router.dart';
import '../edge_swipe_sidebar_toggle.dart';
import 'nav_destinations.dart';
import 'nav_logout.dart';
import 'nav_more_sheets.dart';
import 'shell_content.dart';
import 'sidebar_collapsed_provider.dart';

/// Wide Apple layout: a [CupertinoSidebar] beside the routed page.
class AppleWideLayout extends ConsumerWidget {
  const AppleWideLayout({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final collapsed = ref.watch(sidebarCollapsedProvider).value ?? false;
    final appleSelected = appleSidebarSelectedIndex(
      GoRouterState.of(context).uri.toString(),
    );
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
                    navigationBar: CupertinoSpaceSwitcher(),
                    onDestinationSelected: (i) {
                      final dest = appleSidebarDestinations[i];
                      context.go(dest.path);
                    },
                    children: [
                      // Primary destinations
                      for (final d in iosTabDestinations)
                        SidebarDestination(
                          icon: Icon(d.icon),
                          label: Text(d.label),
                        ),
                      // Overflow section
                      SidebarSection(
                        label: const Text('More'),
                        children: [
                          for (final d in overflowDestinations)
                            SidebarDestination(
                              icon: Icon(d.icon),
                              label: Text(d.label),
                            ),
                        ],
                      ),
                      // Settings
                      SidebarDestination(
                        icon: Icon(settingsDestination.icon),
                        label: Text(settingsDestination.label),
                      ),
                    ],
                  ),
                ),
                Expanded(
                  child: EdgeSwipeSidebarToggle(
                    child: ShellContent(child: child),
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
            // Logout button overlaid at bottom-leading corner of the
            // Cupertino sidebar — mirrors the Material sidebar slot.
            Positioned(
              left: 4,
              bottom: 4,
              child: SafeArea(
                child: CupertinoButton(
                  key: const Key('cupertino-logout'),
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  onPressed: () async {
                    await logoutAndShowLogin(context);
                  },
                  child: const Tooltip(
                    message: 'Log out',
                    child: Icon(
                      CupertinoIcons.square_arrow_right,
                      semanticLabel: 'Log out',
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Compact Apple layout: a [CupertinoTabBar] with a "More" action sheet.
class AppleCompactLayout extends StatelessWidget {
  const AppleCompactLayout({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final selected = iosMobileSelectedIndex(
      GoRouterState.of(context).uri.toString(),
    );
    return Scaffold(
      body: ShellContent(child: child),
      bottomNavigationBar: CupertinoTabBar(
        currentIndex: selected,
        onTap: (i) {
          if (i == iosTabDestinations.length) {
            showCupertinoMoreSheet(context);
          } else {
            context.go(iosTabDestinations[i].path);
          }
        },
        items: [
          for (final d in iosTabDestinations)
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
}

/// Cupertino-styled space switcher used as the [CupertinoSidebar.navigationBar].
///
/// Wraps [SidebarNavigationBar] (which renders a [CupertinoSliverNavigationBar])
/// so it integrates as a proper sliver inside the sidebar's [CustomScrollView].
class CupertinoSpaceSwitcher extends ConsumerWidget {
  const CupertinoSpaceSwitcher({super.key});

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
