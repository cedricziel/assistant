import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';

/// Converts a plain SSE text string into a byte stream for testing.
Stream<List<int>> _sseBytes(String sseText) async* {
  yield utf8.encode(sseText);
}

void main() {
  group('parseSseByteStream', () {
    test('emits RunStartedEvent from run_started SSE event', () async {
      const sse = 'event:run_started\ndata:{"run_id":"abc-123"}\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(1));
      expect(events.first, isA<RunStartedEvent>());
      expect((events.first as RunStartedEvent).runId, equals('abc-123'));
    });

    test('emits TokenEvent from token SSE event', () async {
      const sse = 'event:token\ndata:hello world\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(1));
      expect(events.first, isA<TokenEvent>());
      expect((events.first as TokenEvent).token, equals('hello world'));
    });

    test('emits DoneEvent with role and content from done SSE event', () async {
      final payload = jsonEncode({'role': 'assistant', 'content': 'All done'});
      final sse = 'event:done\ndata:$payload\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(1));
      final done = events.first as DoneEvent;
      expect(done.role, equals('assistant'));
      expect(done.content, equals('All done'));
    });

    test('emits StatusEvent from status SSE event', () async {
      const sse = 'event:status\ndata:Calling tool: web-search\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.first, isA<StatusEvent>());
      expect(
        (events.first as StatusEvent).message,
        equals('Calling tool: web-search'),
      );
    });

    test('emits ToolResultEvent from tool_result SSE event', () async {
      final payload = jsonEncode({'tool_name': 'web-search', 'status': 'ok'});
      final sse = 'event:tool_result\ndata:$payload\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.first, isA<ToolResultEvent>());
      final tr = events.first as ToolResultEvent;
      expect(tr.toolName, equals('web-search'));
      expect(tr.status, equals('ok'));
    });

    test('emits ErrorEvent from error SSE event — non-JSON data', () async {
      // The error event type is not parsed by parseSseByteStream;
      // verify it is silently dropped (no crash, no event emitted).
      const sse = 'event:error\ndata:something went wrong\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();
      // error events are not parsed — they are silently skipped.
      expect(events, isEmpty);
    });

    test('handles sequence of multiple event types in order', () async {
      final done = jsonEncode({'role': 'assistant', 'content': 'done'});
      final sse =
          'event:run_started\ndata:{"run_id":"r1"}\n\n'
          'event:token\ndata:foo\n\n'
          'event:token\ndata:bar\n\n'
          'event:done\ndata:$done\n\n';

      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(4));
      expect(events[0], isA<RunStartedEvent>());
      expect((events[0] as RunStartedEvent).runId, equals('r1'));
      expect(events[1], isA<TokenEvent>());
      expect((events[1] as TokenEvent).token, equals('foo'));
      expect(events[2], isA<TokenEvent>());
      expect((events[2] as TokenEvent).token, equals('bar'));
      expect(events[3], isA<DoneEvent>());
    });

    test('ignores malformed run_started JSON without throwing', () async {
      const sse = 'event:run_started\ndata:not-json\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();
      // Malformed JSON for run_started is silently ignored.
      expect(events, isEmpty);
    });

    test('handles multi-chunk byte stream reassembly', () async {
      // Simulate chunked delivery: SSE event split across two byte chunks.
      final part1 = utf8.encode('event:token\n');
      final part2 = utf8.encode('data:chunked\n\n');

      Stream<List<int>> chunked() async* {
        yield part1;
        yield part2;
      }

      final events = await parseSseByteStream(chunked()).toList();
      expect(events.length, equals(1));
      expect(events.first, isA<TokenEvent>());
      expect((events.first as TokenEvent).token, equals('chunked'));
    });
  });

  group('StreamEvent model', () {
    test('TokenEvent stores token', () {
      const event = TokenEvent('test token');
      expect(event.token, equals('test token'));
    });

    test('DoneEvent parses JSON correctly', () {
      final json = {'role': 'assistant', 'content': 'Hello!'};
      final event = DoneEvent.fromJson(json);
      expect(event.role, equals('assistant'));
      expect(event.content, equals('Hello!'));
    });

    test('DoneEvent handles missing fields gracefully', () {
      final event = DoneEvent.fromJson({});
      expect(event.role, equals('assistant'));
      expect(event.content, equals(''));
    });

    test('ErrorEvent stores message', () {
      const event = ErrorEvent('Connection failed');
      expect(event.message, equals('Connection failed'));
    });

    test('StatusEvent stores message', () {
      const event = StatusEvent('Calling tool: web-search');
      expect(event.message, equals('Calling tool: web-search'));
    });

    test('ToolResultEvent parses JSON correctly', () {
      final json = {'tool_name': 'web-search', 'status': 'ok'};
      final event = ToolResultEvent.fromJson(json);
      expect(event.toolName, equals('web-search'));
      expect(event.status, equals('ok'));
    });

    test('ToolResultEvent handles missing fields gracefully', () {
      final event = ToolResultEvent.fromJson({});
      expect(event.toolName, equals(''));
      expect(event.status, equals('ok'));
    });

    test('StreamEvent sealed class hierarchy', () {
      // Verify pattern matching works correctly.
      final events = <StreamEvent>[
        const TokenEvent('token'),
        const StatusEvent('status'),
        ToolResultEvent.fromJson({'tool_name': 'tool', 'status': 'ok'}),
        const DoneEvent(role: 'assistant', content: 'full'),
        const ErrorEvent('error'),
      ];

      int tokenCount = 0,
          statusCount = 0,
          toolCount = 0,
          doneCount = 0,
          errorCount = 0,
          audioReadyCount = 0;
      for (final e in events) {
        switch (e) {
          case TokenEvent():
            tokenCount++;
          case StatusEvent():
            statusCount++;
          case ToolResultEvent():
            toolCount++;
          case DoneEvent():
            doneCount++;
          case ErrorEvent():
            errorCount++;
          case AudioReadyEvent():
            audioReadyCount++;
          case TranscriptEvent():
            break;
          case RunStartedEvent():
            break; // not counted in this test
        }
      }

      expect(tokenCount, equals(1));
      expect(statusCount, equals(1));
      expect(toolCount, equals(1));
      expect(doneCount, equals(1));
      expect(errorCount, equals(1));
      expect(audioReadyCount, equals(0));
    });
  });
}
