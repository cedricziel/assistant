import 'dart:async';
import 'dart:convert';

import 'package:assistant_ui/models.dart';
import 'package:flutter/foundation.dart';
import 'package:material_ui/material_ui.dart';
import 'package:webview_flutter/webview_flutter.dart';

import 'mcp_app_bridge.dart';
import 'mcp_app_document.dart';
import 'mcp_app_surface.dart';

const _channel = 'McpAppHost';

/// Routes the app's `window.parent.postMessage` to the host channel. In a
/// top-level web view `window.parent` is `window` itself, so without this the
/// app would receive its own messages.
const _bootstrap =
    'window.postMessage = function (message) {'
    ' window.$_channel.postMessage(JSON.stringify(message)); };';

/// Renders [app] in a `webview_flutter` web view on iOS and macOS.
McpAppSurface createPlatformMcpAppSurface(McpApp app) =>
    switch (defaultTargetPlatform) {
      TargetPlatform.iOS || TargetPlatform.macOS => _WebViewSurface(app),
      _ => const _UnsupportedSurface(),
    };

class _WebViewSurface implements McpAppSurface, McpAppPort {
  _WebViewSurface(McpApp app) : this._(app, app.permissions);

  // Takes the permissions apart from the app so the permission callback does
  // not keep the app's HTML alive.
  _WebViewSurface._(McpApp app, McpAppPermissions permissions)
    : _controller = WebViewController(
        onPermissionRequest: (request) => _answer(request, permissions),
      ) {
    _controller
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setNavigationDelegate(
        NavigationDelegate(
          onNavigationRequest: (request) =>
              request.isMainFrame && !request.url.startsWith('about:')
              ? NavigationDecision.prevent
              : NavigationDecision.navigate,
        ),
      )
      ..addJavaScriptChannel(
        _channel,
        onMessageReceived: (message) {
          try {
            if (_incoming.isClosed) return;
            _incoming.add(jsonDecode(message.message));
          } on FormatException {
            // Not a JSON-RPC message.
          }
        },
      );
    unawaited(
      _controller.loadHtmlString(
        mcpAppDocument(app, bootstrapScript: _bootstrap),
      ),
    );
  }

  final WebViewController _controller;
  final _incoming = StreamController<Object?>.broadcast();

  @override
  McpAppPort get port => this;

  @override
  Stream<Object?> get messages => _incoming.stream;

  @override
  void send(Map<String, Object?> message) => unawaited(
    _controller.runJavaScript(
      'window.dispatchEvent(new MessageEvent("message", '
      '{data: ${jsonEncode(message)}, source: window}));',
    ),
  );

  @override
  Widget build(BuildContext context) => WebViewWidget(controller: _controller);

  @override
  void dispose() => unawaited(_incoming.close());

  /// Grants only the device access the app declared.
  static void _answer(
    WebViewPermissionRequest request,
    McpAppPermissions permissions,
  ) {
    final allowed = request.types.every(
      (type) => switch (type) {
        WebViewPermissionResourceType.camera => permissions.camera,
        WebViewPermissionResourceType.microphone => permissions.microphone,
        _ => false,
      },
    );
    unawaited(allowed ? request.grant() : request.deny());
  }
}

class _UnsupportedSurface implements McpAppSurface, McpAppPort {
  const _UnsupportedSurface();

  @override
  McpAppPort get port => this;

  @override
  Stream<Object?> get messages => const Stream.empty();

  @override
  void send(Map<String, Object?> message) {}

  @override
  Widget build(BuildContext context) =>
      const Center(child: Text('This app cannot be shown on this device.'));

  @override
  void dispose() {}
}
