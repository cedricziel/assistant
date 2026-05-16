import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Button that renders Cupertino on Apple touch platforms and Material
/// elsewhere.
///
/// Two variants:
/// - [AdaptiveButton.filled] — primary, emphasized action.
///   iOS: [CupertinoButton.filled]; Material: [FilledButton].
/// - [AdaptiveButton.plain] — secondary, non-emphasized action.
///   iOS: plain [CupertinoButton]; Material: [TextButton].
class AdaptiveButton extends StatelessWidget {
  const AdaptiveButton.filled({
    super.key,
    required this.onPressed,
    required this.child,
  }) : _filled = true;

  const AdaptiveButton.plain({
    super.key,
    required this.onPressed,
    required this.child,
  }) : _filled = false;

  final VoidCallback? onPressed;
  final Widget child;
  final bool _filled;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      if (_filled) {
        return CupertinoButton.filled(onPressed: onPressed, child: child);
      }
      return CupertinoButton(onPressed: onPressed, child: child);
    }
    if (_filled) {
      return FilledButton(onPressed: onPressed, child: child);
    }
    return TextButton(onPressed: onPressed, child: child);
  }
}
