import 'dart:async';
import 'dart:js_interop';

import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';
import 'package:web/web.dart' as web;

import 'mcp_app_bridge.dart';
import 'mcp_app_document.dart';
import 'mcp_app_surface.dart';

/// Renders [app] in an `<iframe sandbox="allow-scripts" srcdoc>`. Without
/// `allow-same-origin` the frame has an opaque origin, so it cannot reach the
/// assistant's cookies, storage or API.
McpAppSurface createPlatformMcpAppSurface(McpApp app) => _IframeSurface(app);

class _IframeSurface implements McpAppSurface, McpAppPort {
  _IframeSurface(McpApp app)
    : _document = mcpAppDocument(app),
      _allow = mcpAppAllow(app.permissions) {
    web.window.addEventListener('message', _listener);
  }

  final String _document;
  final String _allow;
  final _incoming = StreamController<Object?>.broadcast();
  late final JSFunction _listener = _receive.toJS;

  /// Set once Flutter creates the element; the app sends first, so nothing
  /// is posted before then.
  web.HTMLIFrameElement? _iframe;

  void _configure(Object element) {
    final iframe = element as web.HTMLIFrameElement;
    iframe.setAttribute('sandbox', 'allow-scripts');
    if (_allow.isNotEmpty) iframe.allow = _allow;
    iframe.style
      ..border = 'none'
      ..width = '100%'
      ..height = '100%';
    iframe.srcdoc = _document.toJS;
    _iframe = iframe;
  }

  void _receive(web.MessageEvent event) {
    final source = event.source;
    final window = _iframe?.contentWindow;
    if (source == null ||
        window == null ||
        !source.strictEquals(window).toDart) {
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
      _iframe?.contentWindow?.postMessage(message.jsify(), '*'.toJS);

  @override
  Widget build(BuildContext context) => HtmlElementView.fromTagName(
    tagName: 'iframe',
    onElementCreated: _configure,
  );

  @override
  void dispose() {
    web.window.removeEventListener('message', _listener);
    unawaited(_incoming.close());
  }
}
