// RED: openspec/changes/generative-ui-a2ui PR 15.1 — the parser turns a
// `surface_update` frame into a SurfaceUpdateEvent and skips malformed ones.
// PR 17.1 adds the post-turn `surface_action` request and its status errors.

import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/api/models/surface_action.dart';
import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';

Stream<Uint8List> _stream(String sseText) async* {
  yield Uint8List.fromList(utf8.encode(sseText));
}

/// Records the request body and answers with [body] and [status].
class _RecordingAdapter implements HttpClientAdapter {
  _RecordingAdapter(this.body, {this.status = 200, this.headers = const {}});

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

(ApiClient, _RecordingAdapter) _client(
  String body, {
  int status = 200,
  Map<String, List<String>> headers = const {},
}) {
  final adapter = _RecordingAdapter(body, status: status, headers: headers);
  final client = ApiClient(baseUrl: 'http://fake', token: 'tok');
  client.dioForTesting.httpClientAdapter = adapter;
  return (client, adapter);
}

const _sse =
    'event:token\ndata:hi\n\n'
    'event:done\ndata:{"role":"assistant","content":"hi","message_id":"m-2"}\n\n';

const _surface = {
  'surface_id': 'surface-1',
  'tool_call_id': 'call-1',
  'anchor_message_id': 'message-1',
  'anchor_call_index': 0,
  'title': 'Weather',
  'fallback_text': 'Sunny, 21 °C',
  'status': 'active',
  'messages': [
    {
      'version': '1.0',
      'createSurface': {
        'surfaceId': 'surface-1',
        'catalogId': 'https://assistant.invalid/a2ui/catalogs/assistant/v1',
        'components': [
          {'id': 'root', 'component': 'Text', 'text': 'Sunny'},
        ],
      },
    },
  ],
};

void main() {
  group('streamMessages surface_action', () {
    test('posts the action with an empty message', () async {
      final (client, adapter) = _client(
        _sse,
        headers: {
          'x-run-id': ['run-s'],
        },
      );

      final events = await client
          .streamMessages(
            'c-1',
            '',
            surfaceAction: const SurfaceActionSubmission(
              surfaceId: 'surface-1',
              name: 'submit_form',
              sourceComponentId: 'button-1',
              context: {'sku': 'A-1'},
              dataModel: {'quantity': 2},
            ),
          )
          .toList();

      expect(adapter.requests.single.method, 'POST');
      expect(adapter.requests.single.path, '/api/conversations/c-1/messages');
      expect(jsonDecode(adapter.bodies.single), {
        'message': '',
        'surface_action': {
          'surface_id': 'surface-1',
          'name': 'submit_form',
          'source_component_id': 'button-1',
          'context': {'sku': 'A-1'},
          'data_model': {'quantity': 2},
        },
      });
      expect((events.first as RunStartedEvent).runId, 'run-s');
      expect(events.last, isA<DoneEvent>());
    });

    test('a 409 yields an ErrorEvent carrying the status code', () async {
      final (client, _) = _client('{"error":"already submitted"}', status: 409);

      final events = await client
          .streamMessages(
            'c-1',
            '',
            surfaceAction: const SurfaceActionSubmission(
              surfaceId: 'surface-1',
              name: 'submit_form',
              sourceComponentId: 'button-1',
              context: {},
            ),
          )
          .toList();

      final error = events.single as ErrorEvent;
      expect(error.statusCode, 409);
      expect(error.isConflict, isTrue);
    });
  });

  group('parseSseByteStream surface_update', () {
    test('yields a SurfaceUpdateEvent with every field', () async {
      final frame = 'event: surface_update\ndata: ${jsonEncode(_surface)}\n\n';
      final events = await parseSseByteStream(_stream(frame)).toList();

      expect(events, hasLength(1));
      final event = events.single as SurfaceUpdateEvent;
      expect(event.surfaceId, 'surface-1');
      expect(event.toolCallId, 'call-1');
      expect(event.anchorMessageId, 'message-1');
      expect(event.anchorCallIndex, 0);
      expect(event.title, 'Weather');
      expect(event.fallbackText, 'Sunny, 21 °C');
      expect(event.status, 'active');
      expect(event.messages, hasLength(1));
      expect(
        (event.messages.single['createSurface'] as Map)['surfaceId'],
        'surface-1',
      );
    });

    test('keeps the sequence id from the id line', () async {
      final frame =
          'id: 7\nevent: surface_update\ndata: ${jsonEncode(_surface)}\n\n';
      final events = await parseSseByteStream(_stream(frame)).toList();

      expect((events.single as SurfaceUpdateEvent).sequenceId, 7);
    });

    test('a payload without surface_id is skipped', () async {
      final payload = Map<String, Object?>.from(_surface)..remove('surface_id');
      final frame = 'event: surface_update\ndata: ${jsonEncode(payload)}\n\n';

      expect(await parseSseByteStream(_stream(frame)).toList(), isEmpty);
    });

    test('a payload without an anchor is skipped', () async {
      final payload = Map<String, Object?>.from(_surface)
        ..remove('anchor_message_id');
      final frame = 'event: surface_update\ndata: ${jsonEncode(payload)}\n\n';

      expect(await parseSseByteStream(_stream(frame)).toList(), isEmpty);
    });

    test('an unknown event is still ignored', () async {
      const frame = 'event: brand_new\ndata: {"anything":true}\n\n';

      expect(await parseSseByteStream(_stream(frame)).toList(), isEmpty);
    });
  });
}
