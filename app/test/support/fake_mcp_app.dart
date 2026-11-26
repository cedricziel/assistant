import 'dart:async';

import 'package:assistant_ui/models.dart';
import 'package:material_ui/material_ui.dart';

import 'package:assistant_app/features/chat/mcp_app/mcp_app_bridge.dart';
import 'package:assistant_app/features/chat/mcp_app/mcp_app_surface.dart';

/// An app frame's port that records what the host sends.
class FakeMcpAppPort implements McpAppPort {
  final _incoming = StreamController<Object?>.broadcast();
  final sent = <Map<String, Object?>>[];

  /// Delivers [message] as if the app had posted it.
  void fromApp(Map<String, Object?> message) => _incoming.add(message);

  /// Delivers a JSON-RPC request from the app.
  void request(
    Object id,
    String method, [
    Map<String, Object?> params = const {},
  ]) =>
      fromApp({'jsonrpc': '2.0', 'id': id, 'method': method, 'params': params});

  /// Delivers a JSON-RPC notification from the app.
  void notify(String method, [Map<String, Object?> params = const {}]) =>
      fromApp({'jsonrpc': '2.0', 'method': method, 'params': params});

  /// The host's reply to the request with [id].
  Map<String, Object?> replyTo(Object id) =>
      sent.lastWhere((m) => m['id'] == id && m['method'] == null);

  @override
  Stream<Object?> get messages => _incoming.stream;

  @override
  void send(Map<String, Object?> message) => sent.add(message);
}

/// Server routes that answer from fields and record each call.
class FakeMcpAppBackend implements McpAppBackend {
  final calls = <(String, Map<String, Object?>, bool)>[];
  final reads = <String>[];

  /// Refuses every call with this.
  McpAppRefused? refusal;

  /// Answers an unconfirmed call with 409, as for a tool without
  /// `readOnlyHint`.
  bool refuseUnconfirmed = false;

  Map<String, Object?> toolResult = {'content': <Object?>[]};
  Map<String, Object?> resourceResult = {
    'contents': [
      {'uri': 'ui://weather/icon', 'text': 'svg'},
    ],
  };

  @override
  Future<Map<String, Object?>> callTool(
    String name,
    Map<String, Object?> arguments, {
    required bool confirmed,
  }) async {
    calls.add((name, arguments, confirmed));
    if (refusal case final refusal?) throw refusal;
    if (refuseUnconfirmed && !confirmed) {
      throw const McpAppRefused(409, 'needs the user to confirm the call');
    }
    return toolResult;
  }

  @override
  Future<Map<String, Object?>> readResource(String uri) async {
    reads.add(uri);
    if (refusal case final refusal?) throw refusal;
    return resourceResult;
  }
}

/// Key of the placeholder a [FakeMcpAppSurface] renders.
const fakeMcpAppViewKey = ValueKey('fake-mcp-app-view');

/// A platform view stand-in that renders a placeholder over a fake port.
class FakeMcpAppSurface implements McpAppSurface {
  FakeMcpAppSurface(this.app);

  /// Every surface created, most recent last.
  static final created = <FakeMcpAppSurface>[];

  /// Creates surfaces and records them in [created].
  static McpAppSurface factory(McpApp app) {
    final surface = FakeMcpAppSurface(app);
    created.add(surface);
    return surface;
  }

  final McpApp app;
  bool disposed = false;

  @override
  final FakeMcpAppPort port = FakeMcpAppPort();

  @override
  Widget build(BuildContext context) =>
      const SizedBox.expand(key: fakeMcpAppViewKey);

  @override
  void dispose() => disposed = true;
}
