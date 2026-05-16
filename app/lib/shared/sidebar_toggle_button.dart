import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'nav_shell.dart';

/// Toggle button that flips [sidebarCollapsedProvider]. Used as a top-leading
/// overlay on the Material wide branch so the affordance is visible outside
/// the sidebar rail itself.
///
/// Tooltips intentionally differ from the in-sidebar toggle's
/// "Expand sidebar" / "Collapse sidebar" so widget tests can target each
/// independently.
class SidebarToggleButton extends ConsumerWidget {
  const SidebarToggleButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final collapsed = ref.watch(sidebarCollapsedProvider).value ?? false;
    return IconButton(
      icon: Icon(collapsed ? Icons.menu : Icons.menu_open),
      tooltip: collapsed ? 'Show navigation' : 'Hide navigation',
      onPressed: () => ref.read(sidebarCollapsedProvider.notifier).toggle(),
    );
  }
}
