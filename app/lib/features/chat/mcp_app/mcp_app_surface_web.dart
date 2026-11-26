import 'dart:async';
import 'dart:js_interop';
import 'dart:ui_web' as ui_web;

import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:web/web.dart' as web;

import 'mcp_app_bridge.dart';
import 'mcp_app_document.dart';
import 'mcp_app_surface.dart';

int _nextView = 0;

/// Renders [app] in an `<iframe sandbox="allow-scripts" srcdoc>`. Without
/// `allow-same-origin` the frame has an opaque origin, so it cannot reach the
/// assistant's cookies, storage or API.
McpAppSurface createPlatformMcpAppSurface(McpApp app) => _IframeSurface(app);

class _IframeSurface implements McpAppSurface, McpAppPort {
  _IframeSurface(McpApp app) : _viewType = 'mcp-app-${_nextView++}' {
    _iframe.setAttribute('sandbox', 'allow-scripts');
    final allow = mcpAppAllow(app.permissions);
    if (allow.isNotEmpty) _iframe.allow = allow;
    _iframe.style
      ..border = 'none'
      ..width = '100%'
      ..height = '100%';
    _iframe.srcdoc = mcpAppDocument(app).toJS;
    ui_web.platformViewRegistry.registerViewFactory(
      _viewType,
      (int _) => _iframe,
    );
    web.window.addEventListener('message', _listener);
  }

  final String _viewType;
  final _iframe = web.HTMLIFrameElement();
  final _incoming = StreamController<Object?>.broadcast();
  late final JSFunction _listener = _receive.toJS;

  void _receive(web.MessageEvent event) {
    final source = event.source;
    if (source == null || !source.strictEquals(_iframe.contentWindow).toDart) {
      return;
    }
    _incoming.add(event.data.dartify());
  }

  @override
  McpAppPort get port => this;

  @override
  Stream<Object?> get messages => _incoming.stream;

  // An opaque-origin frame matches no target origin but `*`.
  @override
  void send(Map<String, Object?> message) =>
      _iframe.contentWindow?.postMessage(message.jsify(), '*'.toJS);

  @override
  Widget build(BuildContext context) => HtmlElementView(viewType: _viewType);

  @override
  void dispose() {
    web.window.removeEventListener('message', _listener);
    unawaited(_incoming.close());
  }
}
