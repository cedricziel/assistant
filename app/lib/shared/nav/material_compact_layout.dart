import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../../features/pwa/pwa_provider.dart';
import 'install_prompts.dart';
import 'nav_destinations.dart';
import 'nav_more_sheets.dart';
import 'shell_content.dart';

/// Compact Material layout: a bottom [NavigationBar] with a "More" sheet and
/// install-app banners above it.
class MaterialCompactLayout extends ConsumerWidget {
  const MaterialCompactLayout({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isInstallable = ref.watch(pwaInstallProvider);
    final isSafariInstallable = ref.watch(safariInstallProvider);
    final selected = mobileSelectedIndex(
      GoRouterState.of(context).uri.toString(),
    );
    return Scaffold(
      body: ShellContent(child: child),
      bottomNavigationBar: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (isInstallable)
            InstallBanner(
              onInstall: () => ref.read(pwaInstallProvider.notifier).install(),
            ),
          if (isSafariInstallable) const SafariInstallBanner(),
          NavigationBar(
            selectedIndex: selected,
            onDestinationSelected: (i) {
              if (i == primaryDestinations.length) {
                showMaterialMoreSheet(context);
              } else {
                context.go(primaryDestinations[i].path);
              }
            },
            destinations: [
              ...primaryDestinations.map(
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
