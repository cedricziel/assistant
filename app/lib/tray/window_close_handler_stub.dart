import 'package:flutter/material.dart';

/// Pass-through on web / unsupported platforms.
class MacosWindowCloseHandler extends StatelessWidget {
  const MacosWindowCloseHandler({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => child;
}
