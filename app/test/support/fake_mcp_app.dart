import 'dart:async';

import 'package:assistant_app/features/chat/mcp_app/mcp_app_bridge.dart';

/// An app frame's port that records what the host sends.
class FakeMcpAppPort implements McpAppPort {
  final _incoming = StreamController<Object?>.broadcast();
  final sent = <Map<String, Object?>>[];

  /// Delivers [message] as if the app had posted it.
  void fromApp(Map<String, Object?> message) => _incoming.add(message);

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
