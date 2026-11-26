import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_api/assistant_api.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/mcp_app/mcp_app_backend.dart';
import 'package:assistant_app/features/chat/mcp_app/mcp_app_bridge.dart';

class _Adapter implements HttpClientAdapter {
  _Adapter(this.status, this.body);

  final int status;
  final Object body;
  final requests = <(String, Object?)>[];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<dynamic>? cancelFuture,
  ) async {
    final sent = requestStream == null
        ? null
        : jsonDecode(
            utf8.decode(await requestStream.expand((b) => b).toList()),
          );
    requests.add((options.path, sent));
    return ResponseBody.fromBytes(
      utf8.encode(jsonEncode(body)),
      status,
      headers: {
        'content-type': ['application/json'],
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

(ApiMcpAppBackend, _Adapter) _backend(int status, Object body) {
  final adapter = _Adapter(status, body);
  final dio = Dio(BaseOptions(baseUrl: 'http://server'))
    ..httpClientAdapter = adapter;
  return (
    ApiMcpAppBackend(
      ConversationsApi(dio, standardSerializers),
      conversationId: 'conv-1',
      toolCallId: 'call-1',
    ),
    adapter,
  );
}

void main() {
  group('ApiMcpAppBackend', () {
    test('posts a tool call to the tool call\'s app route', () async {
      final (backend, adapter) = _backend(200, {
        'content': [
          {'type': 'text', 'text': 'ok'},
        ],
      });

      final result = await backend.callTool('book', {
        'seat': 3,
      }, confirmed: true);

      expect(result['content'], [
        {'type': 'text', 'text': 'ok'},
      ]);
      final (path, body) = adapter.requests.single;
      expect(
        path,
        '/api/conversations/conv-1/tool-calls/call-1/app/tools-call',
      );
      expect(body, {
        'name': 'book',
        'arguments': {'seat': 3},
        'confirmed': true,
      });
    });

    test('reports a 409 as a refusal that asks for confirmation', () async {
      final (backend, _) = _backend(409, {
        'error': 'needs the user to confirm',
      });

      await expectLater(
        backend.callTool('book', {}, confirmed: false),
        throwsA(
          isA<McpAppRefused>()
              .having((e) => e.statusCode, 'statusCode', 409)
              .having((e) => e.message, 'message', 'needs the user to confirm'),
        ),
      );
    });

    test('reports a 403 with the server\'s reason', () async {
      final (backend, _) = _backend(403, {'error': "not the app's server"});

      await expectLater(
        backend.readResource('ui://other/x'),
        throwsA(
          isA<McpAppRefused>()
              .having((e) => e.statusCode, 'statusCode', 403)
              .having((e) => e.message, 'message', "not the app's server"),
        ),
      );
    });

    test('reads a resource through the app route', () async {
      final (backend, adapter) = _backend(200, {
        'contents': [
          {'uri': 'ui://weather/icon', 'text': 'svg'},
        ],
      });

      final result = await backend.readResource('ui://weather/icon');

      expect(result['contents'], isA<List<Object?>>());
      final (path, body) = adapter.requests.single;
      expect(
        path,
        '/api/conversations/conv-1/tool-calls/call-1/app/resources-read',
      );
      expect(body, {'uri': 'ui://weather/icon'});
    });
  });
}
