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
}
