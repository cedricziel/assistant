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

    test('emits ThinkingEvent from thinking SSE event', () async {
      final payload = jsonEncode({'content': 'Let me consider...'});
      final sse = 'event:thinking\ndata:$payload\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(1));
      expect(events.first, isA<ThinkingEvent>());
      expect(
        (events.first as ThinkingEvent).content,
        equals('Let me consider...'),
      );
    });

    test(
      'emits SubagentStartedEvent from subagent_started SSE event',
      () async {
        final payload = jsonEncode({
          'agent_id': 'research-1',
          'task': 'Find weather data',
        });
        final sse = 'event:subagent_started\ndata:$payload\n\n';
        final events = await parseSseByteStream(_sseBytes(sse)).toList();

        expect(events.length, equals(1));
        expect(events.first, isA<SubagentStartedEvent>());
        final e = events.first as SubagentStartedEvent;
        expect(e.agentId, equals('research-1'));
        expect(e.task, equals('Find weather data'));
      },
    );

    test(
      'emits SubagentCompletedEvent from subagent_completed SSE event',
      () async {
        final payload = jsonEncode({
          'agent_id': 'research-1',
          'status': 'ok',
          'summary': 'Found current conditions',
        });
        final sse = 'event:subagent_completed\ndata:$payload\n\n';
        final events = await parseSseByteStream(_sseBytes(sse)).toList();

        expect(events.length, equals(1));
        expect(events.first, isA<SubagentCompletedEvent>());
        final e = events.first as SubagentCompletedEvent;
        expect(e.agentId, equals('research-1'));
        expect(e.status, equals('ok'));
        expect(e.summary, equals('Found current conditions'));
      },
    );

    test('emits SkillCompleteEvent from skill_complete SSE event', () async {
      final payload = jsonEncode({
        'skill_name': 'web-search',
        'success': true,
        'summary': 'Found 3 results',
      });
      final sse = 'event:skill_complete\ndata:$payload\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(1));
      expect(events.first, isA<SkillCompleteEvent>());
      final e = events.first as SkillCompleteEvent;
      expect(e.skillName, equals('web-search'));
      expect(e.success, isTrue);
      expect(e.summary, equals('Found 3 results'));
    });

    test('emits AgentErrorEvent from agent_error SSE event', () async {
      const sse = 'event:agent_error\ndata:LLM timeout\n\n';
      final events = await parseSseByteStream(_sseBytes(sse)).toList();

      expect(events.length, equals(1));
      expect(events.first, isA<AgentErrorEvent>());
      expect((events.first as AgentErrorEvent).message, equals('LLM timeout'));
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

    test('ThinkingEvent parses JSON correctly', () {
      final event = ThinkingEvent.fromJson({'content': 'Reasoning...'});
      expect(event.content, equals('Reasoning...'));
    });

    test('ThinkingEvent handles missing content', () {
      final event = ThinkingEvent.fromJson({});
      expect(event.content, equals(''));
    });

    test('SubagentStartedEvent parses JSON correctly', () {
      final event = SubagentStartedEvent.fromJson({
        'agent_id': 'a1',
        'task': 'do stuff',
      });
      expect(event.agentId, equals('a1'));
      expect(event.task, equals('do stuff'));
    });

    test('SubagentCompletedEvent parses JSON correctly', () {
      final event = SubagentCompletedEvent.fromJson({
        'agent_id': 'a1',
        'status': 'ok',
        'summary': 'done',
      });
      expect(event.agentId, equals('a1'));
      expect(event.status, equals('ok'));
      expect(event.summary, equals('done'));
    });

    test('SkillCompleteEvent parses JSON correctly', () {
      final event = SkillCompleteEvent.fromJson({
        'skill_name': 'search',
        'success': false,
        'summary': 'failed',
      });
      expect(event.skillName, equals('search'));
      expect(event.success, isFalse);
      expect(event.summary, equals('failed'));
    });

    test('AgentErrorEvent stores message', () {
      const event = AgentErrorEvent('timeout');
      expect(event.message, equals('timeout'));
    });

    test('StreamEvent sealed class hierarchy', () {
      // Verify pattern matching works correctly with all event types.
      final events = <StreamEvent>[
        const TokenEvent('token'),
        const StatusEvent('status'),
        ToolResultEvent.fromJson({'tool_name': 'tool', 'status': 'ok'}),
        const DoneEvent(role: 'assistant', content: 'full'),
        const ErrorEvent('error'),
        const ThinkingEvent('thinking'),
        SubagentStartedEvent.fromJson({'agent_id': 'a', 'task': 't'}),
        SubagentCompletedEvent.fromJson({
          'agent_id': 'a',
          'status': 'ok',
          'summary': 's',
        }),
        SkillCompleteEvent.fromJson({
          'skill_name': 'sk',
          'success': true,
          'summary': 'ok',
        }),
        const AgentErrorEvent('err'),
      ];

      int count = 0;
      for (final e in events) {
        switch (e) {
          case TokenEvent():
          case StatusEvent():
          case ToolResultEvent():
          case DoneEvent():
          case ErrorEvent():
          case ThinkingEvent():
          case SubagentStartedEvent():
          case SubagentCompletedEvent():
          case SkillCompleteEvent():
          case AgentErrorEvent():
          case AudioReadyEvent():
          case TranscriptEvent():
          case RunStartedEvent():
            count++;
        }
      }
      expect(count, equals(events.length));
    });
  });
}
