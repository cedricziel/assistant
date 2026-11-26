import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_timeline_reducers.dart';
import 'package:assistant_ui/models.dart';

void main() {
  group('applyToolResultEvent data', () {
    test('copies data onto the pending record', () {
      final msgs = <ChatMessage>[
        ChatMessage(
          id: 'tc1',
          role: 'assistant',
          content: '',
          timelineType: TimelineEntryType.toolCall,
          toolCalls: [
            ToolCallRecord(
              toolName: 'bash',
              status: ToolCallStatus.pending,
              toolCallId: 'call_1',
            ),
          ],
        ),
      ];

      applyToolResultEvent(
        msgs,
        const ToolResultEvent(
          toolName: 'bash',
          status: 'ok',
          toolCallId: 'call_1',
          data: {'kind': 'terminal', 'exit_code': 0},
        ),
      );

      expect(msgs.single.toolCalls.single.data, {
        'kind': 'terminal',
        'exit_code': 0,
      });
    });

    test('copies data onto a freshly inserted record', () {
      final msgs = <ChatMessage>[];

      applyToolResultEvent(
        msgs,
        const ToolResultEvent(
          toolName: 'web-search',
          status: 'ok',
          data: {'kind': 'sources'},
        ),
      );

      expect(msgs.single.toolCalls.single.data, {'kind': 'sources'});
    });

    test('leaves data null when the event has none', () {
      final msgs = <ChatMessage>[];

      applyToolResultEvent(
        msgs,
        const ToolResultEvent(toolName: 'bash', status: 'ok'),
      );

      expect(msgs.single.toolCalls.single.data, isNull);
    });
  });

  group('applyToolResultEvent app', () {
    const appJson = {
      'server': 'weather',
      'resource_uri': 'ui://weather/forecast',
      'html': '<p>Sunny</p>',
      'csp': {
        'resource_domains': ['https://cdn.example.com'],
      },
      'permissions': {'clipboard_write': true},
    };

    test('parses the app of a tool_result frame', () {
      final event = ToolResultEvent.fromJson({
        'tool_name': 'mcp__weather__forecast',
        'status': 'ok',
        'tool_call_id': 'call_1',
        'app': appJson,
      });

      expect(event.app?.server, 'weather');
      expect(event.app?.csp.resourceDomains, ['https://cdn.example.com']);
      expect(event.app?.permissions.clipboardWrite, isTrue);
    });

    test('copies the app onto the pending record', () {
      final msgs = <ChatMessage>[
        ChatMessage(
          id: 'tc1',
          role: 'assistant',
          content: '',
          timelineType: TimelineEntryType.toolCall,
          toolCalls: [
            ToolCallRecord(
              toolName: 'mcp__weather__forecast',
              status: ToolCallStatus.pending,
              toolCallId: 'call_1',
            ),
          ],
        ),
      ];

      applyToolResultEvent(
        msgs,
        ToolResultEvent.fromJson({
          'tool_name': 'mcp__weather__forecast',
          'status': 'ok',
          'tool_call_id': 'call_1',
          'app': appJson,
        }),
      );

      expect(
        msgs.single.toolCalls.single.app?.resourceUri,
        'ui://weather/forecast',
      );
    });

    test('keeps the app and call id on a freshly inserted record', () {
      final msgs = <ChatMessage>[];

      applyToolResultEvent(
        msgs,
        ToolResultEvent.fromJson({
          'tool_name': 'mcp__weather__forecast',
          'status': 'ok',
          'tool_call_id': 'call_2',
          'app': appJson,
        }),
      );

      final record = msgs.single.toolCalls.single;
      expect(record.app?.server, 'weather');
      expect(record.toolCallId, 'call_2', reason: 'app calls route by it');
    });
  });
}
