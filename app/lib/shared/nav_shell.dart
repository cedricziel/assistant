import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

import '../features/chat/chat_provider.dart';
import 'layout/breakpoints.dart';
import 'nav/apple_layouts.dart';
import 'nav/material_compact_layout.dart';
import 'nav/material_wide_layout.dart';
import 'platform/platform.dart';

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

  @override
  Widget build(BuildContext context) {
    final isWide = MediaQuery.sizeOf(context).width >= Breakpoints.wide;

    // Platform-aware Cmd+N / Ctrl+N shortcut for creating a new chat.
    final useMeta =
        defaultTargetPlatform == TargetPlatform.macOS ||
        defaultTargetPlatform == TargetPlatform.iOS;
    return CallbackShortcuts(
      bindings: {
        SingleActivator(
          LogicalKeyboardKey.keyN,
          meta: useMeta,
          control: !useMeta,
        ): _createNewChat,
      },
      child: Focus(autofocus: true, child: _buildLayout(isWide)),
    );
  }

  Widget _buildLayout(bool isWide) {
    final child = widget.child;
    if (isAppleTouch) {
      return isWide
          ? AppleWideLayout(child: child)
          : AppleCompactLayout(child: child);
    }
    return isWide
        ? MaterialWideLayout(child: child)
        : MaterialCompactLayout(child: child);
  }
}
