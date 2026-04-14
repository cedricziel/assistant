import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/models/stream_event.dart';

void main() {
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
