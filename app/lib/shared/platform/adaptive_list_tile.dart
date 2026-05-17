import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// List tile that renders [CupertinoListTile] on Apple touch platforms and
/// [ListTile] on Material / macOS targets.
///
/// API mirrors the common subset of both underlying tiles: [title],
/// [subtitle], [leading], [trailing], [onTap]. Use inside an
/// [AdaptiveListSection] for the iOS Settings idiom.
class AdaptiveListTile extends StatelessWidget {
  const AdaptiveListTile({
    super.key,
    required this.title,
    this.subtitle,
    this.leading,
    this.trailing,
    this.onTap,
    this.onLongPress,
  });

  final Widget title;
  final Widget? subtitle;
  final Widget? leading;
  final Widget? trailing;
  final VoidCallback? onTap;

  /// Long-press handler. On Material the underlying [ListTile] supports it
  /// natively. On iOS [CupertinoListTile] has no long-press slot, so we wrap
  /// the tile in a [GestureDetector] when a callback is provided.
  final VoidCallback? onLongPress;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      final tile = CupertinoListTile(
        title: title,
        subtitle: subtitle,
        leading: leading,
        trailing: trailing,
        onTap: onTap,
      );
      if (onLongPress == null) return tile;
      return GestureDetector(
        behavior: HitTestBehavior.opaque,
        onLongPress: onLongPress,
        child: tile,
      );
    }
    return ListTile(
      title: title,
      subtitle: subtitle,
      leading: leading,
      trailing: trailing,
      onTap: onTap,
      onLongPress: onLongPress,
    );
  }
}
