import 'package:flutter_test/flutter_test.dart';

import 'package:assistant_app/features/chat/chat_provider.dart';

// Helpers that mirror the private logic in ChatNotifier so we can test it
// without spinning up the full Riverpod graph.

String extractToolName(String statusMessage) {
  const prefix = 'Calling tool: ';
  if (statusMessage.startsWith(prefix)) {
    return statusMessage.substring(prefix.length).trim();
  }
  return statusMessage;
}

ToolCallStatus parseToolStatus(String status) {
  return switch (status) {
    'ok' => ToolCallStatus.ok,
    'error' => ToolCallStatus.error,
    'denied' => ToolCallStatus.denied,
    _ => ToolCallStatus.ok,
  };
}

void main() {
  group('extractToolName', () {
    test('strips "Calling tool: " prefix', () {
      expect(extractToolName('Calling tool: web-search'), 'web-search');
    });

    test('strips prefix with extra whitespace', () {
      expect(extractToolName('Calling tool:  bash '), 'bash');
    });

    test('returns full string when prefix absent', () {
      expect(extractToolName('Thinking...'), 'Thinking...');
    });
  });

  group('parseToolStatus', () {
    test('maps "ok" to ok', () {
      expect(parseToolStatus('ok'), ToolCallStatus.ok);
    });

    test('maps "error" to error', () {
      expect(parseToolStatus('error'), ToolCallStatus.error);
    });

    test('maps "denied" to denied', () {
      expect(parseToolStatus('denied'), ToolCallStatus.denied);
    });

    test('unknown values default to ok', () {
      expect(parseToolStatus('unknown'), ToolCallStatus.ok);
    });
  });

  group('ToolCallRecord accumulation', () {
    test('StatusEvent adds a pending record', () {
      final msg = ChatMessage(
        id: 'assistant-streaming',
        role: 'assistant',
        content: '',
        isStreaming: true,
      );

      final toolName = extractToolName('Calling tool: web-search');
      msg.toolCalls.add(
        ToolCallRecord(toolName: toolName, status: ToolCallStatus.pending),
      );

      expect(msg.toolCalls.length, 1);
      expect(msg.toolCalls.first.toolName, 'web-search');
      expect(msg.toolCalls.first.status, ToolCallStatus.pending);
    });

    test('ToolResultEvent resolves matching pending record', () {
      final msg = ChatMessage(
        id: 'assistant-streaming',
        role: 'assistant',
        content: '',
        isStreaming: true,
      );
      msg.toolCalls.add(
        ToolCallRecord(toolName: 'web-search', status: ToolCallStatus.pending),
      );

      // Simulate ToolResultEvent resolution.
      final callIdx = msg.toolCalls.indexWhere(
        (tc) =>
            tc.toolName == 'web-search' && tc.status == ToolCallStatus.pending,
      );
      expect(callIdx, isNot(-1));
      msg.toolCalls[callIdx].status = parseToolStatus('ok');

      expect(msg.toolCalls.first.status, ToolCallStatus.ok);
    });

    test(
      'ToolResultEvent with no matching pending record appends resolved',
      () {
        final msg = ChatMessage(
          id: 'assistant-streaming',
          role: 'assistant',
          content: '',
          isStreaming: true,
        );

        // No pending record for 'file-read'.
        final callIdx = msg.toolCalls.indexWhere(
          (tc) =>
              tc.toolName == 'file-read' && tc.status == ToolCallStatus.pending,
        );
        expect(callIdx, -1);
        msg.toolCalls.add(
          ToolCallRecord(
            toolName: 'file-read',
            status: parseToolStatus('error'),
          ),
        );

        expect(msg.toolCalls.length, 1);
        expect(msg.toolCalls.first.status, ToolCallStatus.error);
      },
    );

    test('multiple tool calls accumulate in order', () {
      final msg = ChatMessage(
        id: 'assistant-streaming',
        role: 'assistant',
        content: '',
        isStreaming: true,
      );

      msg.toolCalls.add(
        ToolCallRecord(toolName: 'web-search', status: ToolCallStatus.pending),
      );
      msg.toolCalls.add(
        ToolCallRecord(toolName: 'file-read', status: ToolCallStatus.pending),
      );

      // Resolve first.
      msg.toolCalls[0].status = parseToolStatus('ok');
      // Resolve second.
      msg.toolCalls[1].status = parseToolStatus('error');

      expect(msg.toolCalls[0].toolName, 'web-search');
      expect(msg.toolCalls[0].status, ToolCallStatus.ok);
      expect(msg.toolCalls[1].toolName, 'file-read');
      expect(msg.toolCalls[1].status, ToolCallStatus.error);
    });

    test('history tool_calls mapped as resolved ToolCallRecords', () {
      // Simulate what loadConversation does: map ToolCallSummary objects
      // into resolved ToolCallRecords with arguments and result.
      final records = [
        ToolCallRecord(
          toolName: 'web-search',
          status: ToolCallStatus.ok,
          arguments: {'query': 'flutter testing'},
          result: 'Found 10 results.',
        ),
        ToolCallRecord(
          toolName: 'file-read',
          status: ToolCallStatus.error,
          arguments: {'path': '/tmp/missing.txt'},
          result: "Error executing 'file-read': File not found",
        ),
      ];

      final msg = ChatMessage(
        id: 'msg-1',
        role: 'assistant',
        content: 'Here are the results.',
        toolCalls: records,
      );

      expect(msg.toolCalls.length, 2);
      expect(msg.toolCalls[0].toolName, 'web-search');
      expect(msg.toolCalls[0].status, ToolCallStatus.ok);
      expect(msg.toolCalls[0].arguments, {'query': 'flutter testing'});
      expect(msg.toolCalls[0].result, 'Found 10 results.');
      expect(msg.toolCalls[1].toolName, 'file-read');
      expect(msg.toolCalls[1].status, ToolCallStatus.error);
    });

    test('ToolResultEvent updates arguments and result on pending record', () {
      final msg = ChatMessage(
        id: 'assistant-streaming',
        role: 'assistant',
        content: '',
        isStreaming: true,
      );
      msg.toolCalls.add(
        ToolCallRecord(toolName: 'bash', status: ToolCallStatus.pending),
      );

      // Simulate ToolResultEvent resolution with arguments and result.
      msg.toolCalls[0].status = ToolCallStatus.ok;
      msg.toolCalls[0].arguments = {'command': 'ls -la'};
      msg.toolCalls[0].result = 'total 42\ndrwxr-xr-x ...';

      expect(msg.toolCalls[0].status, ToolCallStatus.ok);
      expect(msg.toolCalls[0].arguments, {'command': 'ls -la'});
      expect(msg.toolCalls[0].result, contains('total 42'));
    });

    test('toolCalls preserved on ChatMessage.copyWith', () {
      final record = ToolCallRecord(
        toolName: 'bash',
        status: ToolCallStatus.ok,
      );
      final msg = ChatMessage(
        id: 'a',
        role: 'assistant',
        content: 'hello',
        toolCalls: [record],
      );

      final copied = msg.copyWith(content: 'world');
      expect(copied.toolCalls, same(msg.toolCalls));
      expect(copied.toolCalls.first.toolName, 'bash');
    });
  });
}
