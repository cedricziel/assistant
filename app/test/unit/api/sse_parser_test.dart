// Tests for the SSE byte-stream parser.
//
// Key regression: the parser must accept Stream<Uint8List> (what Dio emits on
// native platforms) without throwing a runtime type error.  Previously the code
// used `.transform(utf8.decoder)` which fails on iOS with:
//   "type 'Utf8Decoder' is not a subtype of type
//    'StreamTransformer<Uint8List, String>'"

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Encodes SSE text as a single [Uint8List] chunk (simulates what Dio emits on
/// native/iOS).
Stream<Uint8List> _uint8Stream(String sseText) async* {
  yield Uint8List.fromList(utf8.encode(sseText));
}

/// Encodes SSE text as a plain [List<int>] chunk (simulates web / older path).
Stream<List<int>> _listIntStream(String sseText) async* {
  yield utf8.encode(sseText);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

void main() {
  group('parseSseByteStream', () {
    const singleToken = 'event: token\ndata: hello\n\n';
    const doneEvent =
        'event: done\ndata: {"role":"assistant","content":"hi"}\n\n';
    const multiChunk = 'event: token\ndata: foo\n\n'
        'event: token\ndata: bar\n\n'
        'event: done\ndata: {"role":"assistant","content":"foobar"}\n\n';

    // -----------------------------------------------------------------------
    // Regression: Uint8List stream must not throw on native platforms
    // -----------------------------------------------------------------------

    test('accepts Stream<Uint8List> without type error', () async {
      final events =
          await parseSseByteStream(_uint8Stream(singleToken)).toList();
      expect(events, hasLength(1));
      expect(events.first, isA<TokenEvent>());
    });

    test('accepts Stream<List<int>>', () async {
      final events =
          await parseSseByteStream(_listIntStream(singleToken)).toList();
      expect(events, hasLength(1));
      expect(events.first, isA<TokenEvent>());
    });

    // -----------------------------------------------------------------------
    // Token event parsing
    // -----------------------------------------------------------------------

    test('emits TokenEvent with correct token', () async {
      final events =
          await parseSseByteStream(_uint8Stream(singleToken)).toList();
      expect((events.first as TokenEvent).token, equals('hello'));
    });

    // -----------------------------------------------------------------------
    // Done event parsing
    // -----------------------------------------------------------------------

    test('emits DoneEvent with role and content from JSON', () async {
      final events =
          await parseSseByteStream(_uint8Stream(doneEvent)).toList();
      expect(events, hasLength(1));
      final done = events.first as DoneEvent;
      expect(done.role, equals('assistant'));
      expect(done.content, equals('hi'));
    });

    test('DoneEvent falls back when data is not valid JSON', () async {
      const raw = 'event: done\ndata: not-json\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, hasLength(1));
      final done = events.first as DoneEvent;
      expect(done.role, equals('assistant'));
      expect(done.content, equals('not-json'));
    });

    // -----------------------------------------------------------------------
    // Multi-event stream
    // -----------------------------------------------------------------------

    test('emits multiple events in order', () async {
      final events =
          await parseSseByteStream(_uint8Stream(multiChunk)).toList();
      expect(events, hasLength(3));
      expect(events[0], isA<TokenEvent>());
      expect((events[0] as TokenEvent).token, equals('foo'));
      expect(events[1], isA<TokenEvent>());
      expect((events[1] as TokenEvent).token, equals('bar'));
      expect(events[2], isA<DoneEvent>());
    });

    // -----------------------------------------------------------------------
    // Multi-chunk stream (data arrives in separate byte chunks)
    // -----------------------------------------------------------------------

    test('handles event split across multiple chunks', () async {
      // Split the SSE message across two chunks, as a network might deliver.
      Stream<Uint8List> splitStream() async* {
        yield Uint8List.fromList(utf8.encode('event: token\n'));
        yield Uint8List.fromList(utf8.encode('data: world\n\n'));
      }

      final events = await parseSseByteStream(splitStream()).toList();
      expect(events, hasLength(1));
      expect((events.first as TokenEvent).token, equals('world'));
    });

    // -----------------------------------------------------------------------
    // Unknown / ignored event types
    // -----------------------------------------------------------------------

    test('ignores unknown event types', () async {
      const raw = 'event: ping\ndata: ignored\n\n';
      final events = await parseSseByteStream(_uint8Stream(raw)).toList();
      expect(events, isEmpty);
    });

    // -----------------------------------------------------------------------
    // Empty stream
    // -----------------------------------------------------------------------

    test('empty stream yields no events', () async {
      final events =
          await parseSseByteStream(const Stream<Uint8List>.empty()).toList();
      expect(events, isEmpty);
    });
  });
}
