import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';

import 'platform.dart';

/// Grouped list section that renders [CupertinoListSection.insetGrouped] on
/// Apple touch platforms and a Material section header + column on
/// Material / macOS targets.
///
/// Compose with [AdaptiveListTile] children. Pass an optional [header] to
/// render an iOS-style section title (or a Material label above the
/// children block).
class AdaptiveListSection extends StatelessWidget {
  const AdaptiveListSection({super.key, this.header, required this.children});

  final Widget? header;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    if (isAppleTouch) {
      return CupertinoListSection.insetGrouped(
        header: header,
        children: children,
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (header != null)
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 20, 16, 4),
            child: DefaultTextStyle.merge(
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                color: Theme.of(context).colorScheme.primary,
              ),
              child: header!,
            ),
          ),
        ...children,
      ],
    );
  }
}
