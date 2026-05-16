import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Row combining a label, optional subtitle, and a switch.
///
/// On Apple touch platforms renders a [CupertinoListTile] with a trailing
/// [CupertinoSwitch] (the iOS Settings idiom). On Material / macOS renders
/// a [SwitchListTile].
class AdaptiveSwitchTile extends StatelessWidget {
  const AdaptiveSwitchTile({
    super.key,
    required this.title,
    this.subtitle,
    this.leading,
    required this.value,
    required this.onChanged,
  });

  final Widget title;
  final Widget? subtitle;
  final Widget? leading;
  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoListTile(
        leading: leading,
        title: title,
        subtitle: subtitle,
        trailing: CupertinoSwitch(value: value, onChanged: onChanged),
      );
    }
    return SwitchListTile(
      secondary: leading,
      title: title,
      subtitle: subtitle,
      value: value,
      onChanged: onChanged,
    );
  }
}
