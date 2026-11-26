// Tests that `tool_result` SSE frames carry structured `data` into
// [ToolResultEvent], and that `subagent_tool_result` frames are unaffected.

import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/models/stream_event.dart';

Future<List<StreamEvent>> _parse(String sse) =>
    parseSseByteStream(Stream.value(utf8.encode(sse))).toList();

void main() {
  group('tool_result frame data', () {
    test('object data reaches ToolResultEvent.data', () async {
      final events = await _parse(
        'event: tool_result\n'
        'data: {"tool_name":"bash","status":"ok","result":"hi",'
        '"tool_call_id":"call_1","data":{"kind":"terminal","exit_code":0}}\n\n',
      );
      final event = events.single as ToolResultEvent;
      expect(event.data, {'kind': 'terminal', 'exit_code': 0});
      expect(event.toolCallId, 'call_1');
    });

    test('absent data leaves ToolResultEvent.data null', () async {
      final events = await _parse(
        'event: tool_result\n'
        'data: {"tool_name":"bash","status":"ok","result":"hi"}\n\n',
      );
      expect((events.single as ToolResultEvent).data, isNull);
    });

    test('string data is ignored and the event is still emitted', () async {
      final events = await _parse(
        'event: tool_result\n'
        'data: {"tool_name":"bash","status":"ok","result":"hi",'
        '"data":"not an object"}\n\n',
      );
      final event = events.single as ToolResultEvent;
      expect(event.data, isNull);
      expect(event.toolName, 'bash');
      expect(event.result, 'hi');
    });

    test('ToolResultEvent.fromJson parses object data', () {
      final event = ToolResultEvent.fromJson({
        'tool_name': 'bash',
        'status': 'ok',
        'data': {'kind': 'terminal'},
      });
      expect(event.data, {'kind': 'terminal'});
    });

    test('ToolResultEvent.fromJson ignores non-object data', () {
      final event = ToolResultEvent.fromJson({
        'tool_name': 'bash',
        'status': 'ok',
        'data': [1, 2],
      });
      expect(event.data, isNull);
    });
  });

  group('subagent_tool_result frame', () {
    test('parses as before and carries no data', () async {
      final events = await _parse(
        'event: subagent_tool_result\n'
        'data: {"agent_id":"a1","data":{"tool_name":"bash","status":"ok",'
        '"result":"hi","data":{"kind":"terminal"}}}\n\n',
      );
      final event = events.single as SubagentToolResultEvent;
      expect(event.agentId, 'a1');
      expect(event.toolName, 'bash');
      expect(event.result, 'hi');
    });
  });
}
