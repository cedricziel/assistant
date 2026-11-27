// RED: openspec/changes/generative-ui-a2ui PR 15.1 — the parser turns a
// `surface_update` frame into a SurfaceUpdateEvent and skips malformed ones.

import 'dart:convert';
import 'dart:typed_data';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';
import 'package:flutter_test/flutter_test.dart';

Stream<Uint8List> _stream(String sseText) async* {
  yield Uint8List.fromList(utf8.encode(sseText));
}

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
