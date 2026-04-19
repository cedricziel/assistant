import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Scaffold that renders [CupertinoPageScaffold] on Apple touch platforms
/// and [Scaffold] on Material platforms.
///
/// Pass [navigationBar] for the Cupertino path and [appBar] for the
/// Material path. When using [AdaptiveNavBar], it implements
/// [ObstructingPreferredSizeWidget] and can serve as both.
class AdaptiveScaffold extends StatelessWidget {
  const AdaptiveScaffold({
    super.key,
    this.navigationBar,
    this.appBar,
    required this.body,
    this.floatingActionButton,
  });

  /// Navigation bar for the Cupertino path.
  final ObstructingPreferredSizeWidget? navigationBar;

  /// App bar for the Material path.
  final PreferredSizeWidget? appBar;

  /// The primary content of the scaffold.
  final Widget body;

  /// Floating action button (Material only).
  final Widget? floatingActionButton;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      if (navigationBar != null) {
        return CupertinoPageScaffold(
          navigationBar: navigationBar!,
          child: body,
        );
      }
      return CupertinoPageScaffold(child: body);
    }
    return Scaffold(
      appBar: appBar,
      body: body,
      floatingActionButton: floatingActionButton,
    );
  }
}
