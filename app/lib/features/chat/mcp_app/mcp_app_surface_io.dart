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

class _WebViewSurface implements McpAppSurface {
  _WebViewSurface(McpApp app) {
    final permissions = app.permissions;
    _controller =
        WebViewController(
            onPermissionRequest: (request) {
              final allowed = request.types.every(
                (type) => switch (type) {
                  WebViewPermissionResourceType.camera => permissions.camera,
                  WebViewPermissionResourceType.microphone =>
                    permissions.microphone,
                  _ => false,
                },
              );
              unawaited(allowed ? request.grant() : request.deny());
            },
          )
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
                _port._incoming.add(jsonDecode(message.message));
              } on FormatException {
                // Not a JSON-RPC message.
              }
            },
          );
    _port = _WebViewPort(_controller);
    unawaited(
      _controller.loadHtmlString(
        mcpAppDocument(app, bootstrapScript: _bootstrap),
      ),
    );
  }

  late final WebViewController _controller;
  late final _WebViewPort _port;

  @override
  McpAppPort get port => _port;

  @override
  Widget build(BuildContext context) => WebViewWidget(controller: _controller);

  @override
  void dispose() => unawaited(_port._incoming.close());
}

class _WebViewPort implements McpAppPort {
  _WebViewPort(this._controller);

  final WebViewController _controller;
  final _incoming = StreamController<Object?>.broadcast();

  @override
  Stream<Object?> get messages => _incoming.stream;

  @override
  void send(Map<String, Object?> message) => unawaited(
    _controller.runJavaScript(
      'window.dispatchEvent(new MessageEvent("message", '
      '{data: ${jsonEncode(message)}, source: window}));',
    ),
  );
}

class _UnsupportedSurface implements McpAppSurface {
  const _UnsupportedSurface();

  @override
  McpAppPort get port => const _ClosedPort();

  @override
  Widget build(BuildContext context) =>
      const Center(child: Text('This app cannot be shown on this device.'));

  @override
  void dispose() {}
}

class _ClosedPort implements McpAppPort {
  const _ClosedPort();

  @override
  Stream<Object?> get messages => const Stream.empty();

  @override
  void send(Map<String, Object?> message) {}
}
