import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Sliver navigation bar that renders [CupertinoSliverNavigationBar] (with
/// large title) on Apple touch platforms and [SliverAppBar] on Material /
/// macOS targets.
///
/// Use this widget inside a [CustomScrollView] for list-style screens that
/// want the iOS large-title-collapses-on-scroll pattern. On Material the
/// equivalent is a [SliverAppBar.large] which provides similar behaviour.
///
/// On iOS 26 the underlying Cupertino sliver will be swapped for an
/// `adaptive_platform_ui` UIKit-embedded native nav bar in a later phase;
/// the wrapper API stays unchanged.
class AdaptiveSliverNavBar extends StatelessWidget {
  const AdaptiveSliverNavBar({
    super.key,
    required this.title,
    this.actions,
    this.leading,
    this.previousPageTitle,
  });

  final String title;
  final List<Widget>? actions;
  final Widget? leading;
  final String? previousPageTitle;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoSliverNavigationBar(
        largeTitle: Text(title),
        leading: leading,
        previousPageTitle: previousPageTitle,
        trailing: (actions != null && actions!.isNotEmpty)
            ? Row(mainAxisSize: MainAxisSize.min, children: actions!)
            : null,
      );
    }
    return SliverAppBar.large(
      title: Text(title),
      leading: leading,
      actions: actions,
    );
  }
}
