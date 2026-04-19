import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Navigation bar that renders [CupertinoNavigationBar] on Apple touch
/// platforms and [AppBar] on Material platforms.
///
/// Implements [ObstructingPreferredSizeWidget] so it can be used as both
/// [Scaffold.appBar] and [CupertinoPageScaffold.navigationBar].
class AdaptiveNavBar extends StatelessWidget
    implements ObstructingPreferredSizeWidget {
  const AdaptiveNavBar({
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
  Size get preferredSize => isAppleTouch
      ? const Size.fromHeight(44)
      : const Size.fromHeight(kToolbarHeight);

  @override
  bool shouldFullyObstruct(BuildContext context) => true;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoNavigationBar(
        middle: Text(title),
        leading: leading,
        previousPageTitle: previousPageTitle,
        trailing: actions != null && actions!.isNotEmpty
            ? Row(mainAxisSize: MainAxisSize.min, children: actions!)
            : null,
      );
    }
    return AppBar(title: Text(title), leading: leading, actions: actions);
  }
}
