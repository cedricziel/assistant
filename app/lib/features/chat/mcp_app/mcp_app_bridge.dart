import 'dart:async';

import 'package:assistant_ui/models.dart';

/// A bidirectional channel to one app frame, carrying JSON-RPC messages.
abstract interface class McpAppPort {
  Stream<Object?> get messages;
  void send(Map<String, Object?> message);
}

/// The server routes that serve an app's own calls.
abstract interface class McpAppBackend {
  Future<Map<String, Object?>> callTool(
    String name,
    Map<String, Object?> arguments, {
    required bool confirmed,
  });

  Future<Map<String, Object?>> readResource(String uri);
}

/// The server refused an app call; [statusCode] is the HTTP status.
class McpAppRefused implements Exception {
  const McpAppRefused(this.statusCode, this.message);

  final int? statusCode;
  final String message;

  @override
  String toString() => message;
}

/// What the host tells the app about where it is rendered.
class McpAppHostContext {
  const McpAppHostContext({
    required this.theme,
    required this.locale,
    required this.timeZone,
    required this.platform,
  });

  /// `light` or `dark`.
  final String theme;
  final String locale;
  final String timeZone;

  /// `web`, `desktop` or `mobile`.
  final String platform;
}

/// The MCP Apps protocol version this host speaks.
const mcpAppsProtocolVersion = '2026-01-26';

const _methodNotFound = -32601;
const _invalidParams = -32602;
const _serverError = -32000;

/// The host side of the MCP Apps postMessage JSON-RPC protocol for one frame.
///
/// Nothing is sent to the app before it reports
/// `ui/notifications/initialized`, except replies to its own requests. The
/// tool input and the tool result (or cancellation) follow once, in that
/// order.
class McpAppBridge {
  McpAppBridge({
    required this.port,
    required this.backend,
    required this.toolCall,
    required this.hostContext,
    required this.confirmToolCall,
    required this.confirmOpenLink,
    required this.openLink,
    required this.onSizeChanged,
    this.hostVersion = 'unknown',
  }) {
    _subscription = port.messages.listen(_receive);
  }

  final McpAppPort port;
  final McpAppBackend backend;
  final ToolCallRecord toolCall;
  final McpAppHostContext hostContext;

  /// Asks the user whether the app may run a tool the server will not run
  /// unconfirmed.
  final Future<bool> Function(String name, Map<String, Object?> arguments)
  confirmToolCall;

  /// Asks the user whether to open a link the app requested.
  final Future<bool> Function(Uri url) confirmOpenLink;
  final Future<void> Function(Uri url) openLink;
  final void Function(double height) onSizeChanged;
  final String hostVersion;

  late final StreamSubscription<Object?> _subscription;
  bool _initialized = false;
  bool _closed = false;
  int _nextId = 0;
  final _pending = <int, Completer<void>>{};

  /// Sends `ui/resource-teardown`, waits up to [timeout] for the app's reply,
  /// then stops listening.
  Future<void> close({
    Duration timeout = const Duration(milliseconds: 500),
  }) async {
    if (_closed) return;
    final id = _nextId++;
    final reply = Completer<void>();
    _pending[id] = reply;
    port.send({
      'jsonrpc': '2.0',
      'id': id,
      'method': 'ui/resource-teardown',
      'params': {'reason': 'The chat no longer shows this app.'},
    });
    try {
      await reply.future.timeout(timeout);
    } on TimeoutException {
      // The app keeps no say in its removal.
    } finally {
      _closed = true;
      _pending.clear();
      await _subscription.cancel();
    }
  }

  void _receive(Object? message) {
    if (_closed || message is! Map) return;
    final method = message['method'];
    final id = message['id'];
    final params = message['params'] is Map
        ? (message['params'] as Map).cast<String, Object?>()
        : const <String, Object?>{};
    if (method is! String) {
      if (id is int) _pending.remove(id)?.complete();
      return;
    }
    if (id == null) {
      _notification(method, params);
    } else {
      unawaited(_request(id, method, params));
    }
  }

  void _notification(String method, Map<String, Object?> params) {
    switch (method) {
      case 'ui/notifications/initialized':
        if (_initialized) return;
        _initialized = true;
        _sendToolCall();
      case 'ui/notifications/size-changed':
        if (params['height'] case final num height) {
          onSizeChanged(height.toDouble());
        }
    }
  }

  void _sendToolCall() {
    _notify('ui/notifications/tool-input', {
      'arguments': toolCall.arguments ?? const <String, Object?>{},
    });
    if (toolCall.status == ToolCallStatus.cancelled) {
      _notify('ui/notifications/tool-cancelled', {
        'reason': 'The tool call was cancelled.',
      });
      return;
    }
    _notify('ui/notifications/tool-result', {
      'content': [
        if (toolCall.result case final text?) {'type': 'text', 'text': text},
      ],
      'structuredContent': ?toolCall.data,
      'isError': toolCall.status != ToolCallStatus.ok,
    });
  }

  Future<void> _request(
    Object id,
    String method,
    Map<String, Object?> params,
  ) async {
    try {
      final result = await switch (method) {
        'ui/initialize' => Future.value(_initializeResult()),
        'ping' => Future.value(const <String, Object?>{}),
        'ui/open-link' => _openLink(params),
        'tools/call' => _callTool(params),
        'resources/read' => _readResource(params),
        _ => throw _RpcError(_methodNotFound, 'Method not found: $method'),
      };
      _reply(id, {'result': result});
    } on _RpcError catch (e) {
      _reply(id, {
        'error': {'code': e.code, 'message': e.message},
      });
    } on McpAppRefused catch (e) {
      _reply(id, {
        'error': {'code': _serverError, 'message': e.message},
      });
    } catch (e) {
      _reply(id, {
        'error': {'code': _serverError, 'message': '$e'},
      });
    }
  }

  Map<String, Object?> _initializeResult() => {
    'protocolVersion': mcpAppsProtocolVersion,
    'hostCapabilities': {
      'openLinks': <String, Object?>{},
      'serverTools': <String, Object?>{},
      'serverResources': <String, Object?>{},
    },
    'hostInfo': {'name': 'assistant', 'version': hostVersion},
    'hostContext': {
      'theme': hostContext.theme,
      'displayMode': 'inline',
      'availableDisplayModes': ['inline'],
      'locale': hostContext.locale,
      'timeZone': hostContext.timeZone,
      'platform': hostContext.platform,
    },
  };

  Future<Map<String, Object?>> _openLink(Map<String, Object?> params) async {
    final url = Uri.tryParse(params['url'] as String? ?? '');
    if (url == null || !(url.isScheme('https') || url.isScheme('http'))) {
      throw const _RpcError(_invalidParams, 'Only http and https links open');
    }
    if (!await confirmOpenLink(url)) {
      throw const _RpcError(_serverError, 'The user declined the link');
    }
    await openLink(url);
    return const {};
  }

  Future<Map<String, Object?>> _callTool(Map<String, Object?> params) async {
    final name = params['name'];
    if (name is! String) {
      throw const _RpcError(_invalidParams, 'tools/call needs a name');
    }
    final arguments = params['arguments'] is Map
        ? (params['arguments'] as Map).cast<String, Object?>()
        : <String, Object?>{};
    try {
      return await backend.callTool(name, arguments, confirmed: false);
    } on McpAppRefused catch (e) {
      if (e.statusCode != 409) rethrow;
    }
    if (!await confirmToolCall(name, arguments)) {
      throw const _RpcError(_serverError, 'The user declined the tool call');
    }
    return backend.callTool(name, arguments, confirmed: true);
  }

  Future<Map<String, Object?>> _readResource(Map<String, Object?> params) {
    final uri = params['uri'];
    if (uri is! String) {
      throw const _RpcError(_invalidParams, 'resources/read needs a uri');
    }
    return backend.readResource(uri);
  }

  void _notify(String method, Map<String, Object?> params) =>
      port.send({'jsonrpc': '2.0', 'method': method, 'params': params});

  void _reply(Object id, Map<String, Object?> body) {
    if (_closed) return;
    port.send({'jsonrpc': '2.0', 'id': id, ...body});
  }
}

class _RpcError implements Exception {
  const _RpcError(this.code, this.message);

  final int code;
  final String message;
}
