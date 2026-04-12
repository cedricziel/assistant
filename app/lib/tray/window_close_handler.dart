import 'dart:io';

import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

/// Wraps the widget tree on macOS to intercept window close events and hide
/// the window instead of quitting, keeping the tray icon active.
///
/// On non-macOS targets this is a transparent pass-through.
class MacosWindowCloseHandler extends StatefulWidget {
  const MacosWindowCloseHandler({super.key, required this.child});

  final Widget child;

  @override
  State<MacosWindowCloseHandler> createState() =>
      _MacosWindowCloseHandlerState();
}

class _MacosWindowCloseHandlerState extends State<MacosWindowCloseHandler>
    with WindowListener {
  @override
  void initState() {
    super.initState();
    if (!Platform.isMacOS) return;
    windowManager.addListener(this);
    windowManager.setPreventClose(true);
  }

  @override
  void dispose() {
    if (Platform.isMacOS) windowManager.removeListener(this);
    super.dispose();
  }

  @override
  void onWindowClose() async {
    if (!Platform.isMacOS) return;
    await windowManager.hide();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
