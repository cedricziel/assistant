import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Switch that renders [CupertinoSwitch] on Apple touch platforms and
/// Material [Switch] on Material / macOS targets.
///
/// [value] is the current state. [onChanged] is the toggle callback;
/// pass `null` to render the switch as disabled.
class AdaptiveSwitch extends StatelessWidget {
  const AdaptiveSwitch({
    super.key,
    required this.value,
    required this.onChanged,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoSwitch(value: value, onChanged: onChanged);
    }
    return Switch(value: value, onChanged: onChanged);
  }
}
