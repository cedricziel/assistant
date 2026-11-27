// Tests for the edit and regenerate streams, and the typed 404/409 errors the
// branch-aware routes surface, plus `user_message_id` on `run_started`.

import 'dart:convert';
import 'dart:typed_data';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';

/// Records the request and answers with [status], [body] and [headers].
class _Adapter implements HttpClientAdapter {
  _Adapter(this.body, {this.status = 200, this.headers = const {}});

  final String body;
  final int status;
  final Map<String, List<String>> headers;
  final List<RequestOptions> requests = [];
  final List<String> bodies = [];

  @override
  Future<ResponseBody> fetch(
    RequestOptions options,
    Stream<Uint8List>? requestStream,
    Future<dynamic>? cancelFuture,
  ) async {
    requests.add(options);
    final sent = <int>[];
    if (requestStream != null) {
      await for (final chunk in requestStream) {
        sent.addAll(chunk);
      }
    }
    bodies.add(utf8.decode(sent));
    return ResponseBody.fromBytes(
      utf8.encode(body),
      status,
      headers: {
        'content-type': [
          status == 200 ? 'text/event-stream' : 'application/json',
        ],
        ...headers,
      },
    );
  }

  @override
  void close({bool force = false}) {}
}

const _sse =
    'event:token\ndata:hi\n\n'
    'event:done\ndata:{"role":"assistant","content":"hi","message_id":"m-2"}\n\n';

(ApiClient, _Adapter) _client(
  String body, {
  int status = 200,
  Map<String, List<String>> headers = const {},
}) {
  final adapter = _Adapter(body, status: status, headers: headers);
  final client = ApiClient(baseUrl: 'http://fake', token: 'tok');
  client.dioForTesting.httpClientAdapter = adapter;
  return (client, adapter);
}

void main() {
  group('RunStartedEvent.userMessageId', () {
    test('parses the optional user_message_id', () async {
      const sse =
          'event:run_started\ndata:{"run_id":"r1","user_message_id":"u-9"}\n\n';
      final events = await parseSseByteStream(Stream.value(utf8.encode(sse)))
          .toList();

      final started = events.single as RunStartedEvent;
      expect(started.runId, 'r1');
      expect(started.userMessageId, 'u-9');
    });

    test('is null when absent and unknown fields are ignored', () async {
      const sse =
          'event:run_started\ndata:{"run_id":"r1","future":{"x":1}}\n\n';
      final events = await parseSseByteStream(Stream.value(utf8.encode(sse)))
          .toList();

      final started = events.single as RunStartedEvent;
      expect(started.runId, 'r1');
      expect(started.userMessageId, isNull);
    });
  });

  group('streamEdit', () {
    test('posts the edited text and yields run id then frames', () async {
      final (client, adapter) = _client(
        _sse,
        headers: {
          'x-run-id': ['run-e'],
        },
      );

      final events = await client.streamEdit('c-1', 'm-1', 'new text').toList();

      expect(adapter.requests.single.method, 'POST');
      expect(
        adapter.requests.single.path,
        '/api/conversations/c-1/messages/m-1/edit',
      );
      expect(jsonDecode(adapter.bodies.single), {'message': 'new text'});
      expect((events.first as RunStartedEvent).runId, 'run-e');
      expect(events[1], isA<TokenEvent>());
      expect(events.last, isA<DoneEvent>());
    });

    test('surfaces a 409 as a conflict error', () async {
      final (client, _) = _client('{"error":"busy"}', status: 409);

      final events = await client.streamEdit('c-1', 'm-1', 'x').toList();

      final error = events.single as ErrorEvent;
      expect(error.statusCode, 409);
      expect(error.isConflict, isTrue);
    });
  });

  group('streamRegenerate', () {
    test('posts an empty body without a model', () async {
      final (client, adapter) = _client(
        _sse,
        headers: {
          'x-run-id': ['run-r'],
        },
      );

      final events = await client.streamRegenerate('c-1', 'm-2').toList();

      expect(
        adapter.requests.single.path,
        '/api/conversations/c-1/messages/m-2/regenerate',
      );
      expect(jsonDecode(adapter.bodies.single), <String, dynamic>{});
      expect((events.first as RunStartedEvent).runId, 'run-r');
      expect(events.last, isA<DoneEvent>());
    });

    test('sends the model override', () async {
      final (client, adapter) = _client(_sse);

      await client.streamRegenerate('c-1', 'm-2', model: 'big-model').toList();

      expect(jsonDecode(adapter.bodies.single), {'model': 'big-model'});
    });

    test('surfaces a 404 as a not-found error', () async {
      final (client, _) = _client('{"error":"gone"}', status: 404);

      final events = await client.streamRegenerate('c-1', 'm-2').toList();

      final error = events.single as ErrorEvent;
      expect(error.statusCode, 404);
      expect(error.isNotFound, isTrue);
    });
  });

  group('streamMessages', () {
    test('surfaces a 409 as a conflict error', () async {
      final (client, _) = _client('{"error":"busy"}', status: 409);

      final events = await client.streamMessages('c-1', 'hello').toList();

      final error = events.single as ErrorEvent;
      expect(error.isConflict, isTrue);
      expect(error.isNotFound, isFalse);
    });
  });
}
