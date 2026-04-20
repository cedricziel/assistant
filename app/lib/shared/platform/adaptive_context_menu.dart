import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// A single action for [AdaptiveMediaContextMenu].
class AdaptiveContextMenuAction {
  const AdaptiveContextMenuAction({
    required this.label,
    required this.onPressed,
    this.cupertinoIcon,
    this.materialIcon,
    this.isDestructive = false,
  });

  final String label;
  final VoidCallback onPressed;
  final IconData? cupertinoIcon;
  final IconData? materialIcon;
  final bool isDestructive;
}

/// Platform-adaptive context menu for media content (images, audio).
///
/// - Apple touch (iOS/iPadOS): [CupertinoContextMenu] with lifted preview.
/// - macOS: [CupertinoContextMenu] (right-click also triggers).
/// - Material/web: long-press or right-click → [showMenu] popup.
class AdaptiveMediaContextMenu extends StatelessWidget {
  const AdaptiveMediaContextMenu({
    super.key,
    required this.child,
    required this.actions,
  });

  final Widget child;
  final List<AdaptiveContextMenuAction> actions;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch || isMacOS) {
      return CupertinoContextMenu(
        actions: actions
            .map(
              (a) => CupertinoContextMenuAction(
                onPressed: () {
                  Navigator.of(context, rootNavigator: true).pop();
                  a.onPressed();
                },
                trailingIcon: a.cupertinoIcon,
                isDestructiveAction: a.isDestructive,
                child: Text(a.label),
              ),
            )
            .toList(),
        child: child,
      );
    }

    // Material / web fallback: long-press or right-click opens a popup menu.
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onLongPressStart: (details) =>
          _showMaterialMenu(context, details.globalPosition),
      onSecondaryTapUp: (details) =>
          _showMaterialMenu(context, details.globalPosition),
      child: child,
    );
  }

  void _showMaterialMenu(BuildContext context, Offset position) {
    final overlay = Overlay.of(context).context.findRenderObject() as RenderBox;
    showMenu<void>(
      context: context,
      position: RelativeRect.fromLTRB(
        position.dx,
        position.dy,
        overlay.size.width - position.dx,
        overlay.size.height - position.dy,
      ),
      items: actions
          .map(
            (a) => PopupMenuItem<void>(
              onTap: a.onPressed,
              child: Row(
                children: [
                  if (a.materialIcon != null) ...[
                    Icon(a.materialIcon, size: 20),
                    const SizedBox(width: 12),
                  ],
                  Text(
                    a.label,
                    style: a.isDestructive
                        ? TextStyle(color: Theme.of(context).colorScheme.error)
                        : null,
                  ),
                ],
              ),
            ),
          )
          .toList(),
    );
  }
}
