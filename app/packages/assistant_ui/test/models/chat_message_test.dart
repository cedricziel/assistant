import 'dart:typed_data';

import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  group('ChatMessage', () {
    test('defaults match the pre-move model', () {
      final m = ChatMessage(id: 'a', role: 'assistant', content: 'hi');
      expect(m.isStreaming, isFalse);
      expect(m.isStale, isFalse);
      expect(m.status, MessageStatus.ok);
      expect(m.timelineType, TimelineEntryType.message);
      expect(m.toolCalls, isEmpty);
      expect(m.attachments, isEmpty);
      expect(m.isAssistant, isTrue);
      expect(m.isUser, isFalse);
    });

    test('content, isStreaming, isStale and status mutate in place', () {
      final m = ChatMessage(id: 'a', role: 'assistant', content: '');
      m
        ..content = 'partial'
        ..isStreaming = true
        ..isStale = true
        ..status = MessageStatus.stopped;
      expect(m.content, 'partial');
      expect(m.isStreaming, isTrue);
      expect(m.isStale, isTrue);
      expect(m.status, MessageStatus.stopped);
    });

    test('copyWith overrides given fields and keeps the rest', () {
      final tool = ToolCallRecord(toolName: 't', status: ToolCallStatus.ok);
      final m = ChatMessage(
        id: 'a',
        role: 'assistant',
        content: 'one',
        toolCalls: [tool],
      );
      final c = m.copyWith(content: 'two', status: MessageStatus.failed);
      expect(c.id, 'a');
      expect(c.content, 'two');
      expect(c.status, MessageStatus.failed);
      expect(c.toolCalls, [tool]);
    });

    test('copyWith clearTokenStream drops the stream', () {
      final m = ChatMessage(
        id: 'a',
        role: 'assistant',
        content: '',
        tokenStream: const Stream<String>.empty(),
      );
      expect(m.copyWith(clearTokenStream: true).tokenStream, isNull);
      expect(m.copyWith().tokenStream, isNotNull);
    });
  });

  group('ToolCallRecord', () {
    test('status mutates in place', () {
      final t = ToolCallRecord(toolName: 'x', status: ToolCallStatus.pending);
      t.status = ToolCallStatus.cancelled;
      expect(t.status, ToolCallStatus.cancelled);
    });
  });

  group('PendingAttachment', () {
    test('holds bytes, filename and mime type', () {
      final a = PendingAttachment(
        bytes: Uint8List.fromList([1, 2]),
        filename: 'a.png',
        mimeType: 'image/png',
      );
      expect(a.bytes, [1, 2]);
      expect(a.filename, 'a.png');
      expect(a.mimeType, 'image/png');
    });
  });

  group('commands and density', () {
    test('CommandEntry.hasArgs follows args', () {
      const none = CommandEntry(name: 'a', description: 'd', category: 'c');
      const some = CommandEntry(
        name: 'a',
        description: 'd',
        category: 'c',
        args: [CommandArg(name: 'x', required: true)],
      );
      expect(none.hasArgs, isFalse);
      expect(some.hasArgs, isTrue);
    });

    test('TimelineDensity.fromWidth uses the 400 and 700 breakpoints', () {
      expect(TimelineDensity.fromWidth(399), TimelineDensity.compact);
      expect(TimelineDensity.fromWidth(400), TimelineDensity.normal);
      expect(TimelineDensity.fromWidth(700), TimelineDensity.normal);
      expect(TimelineDensity.fromWidth(701), TimelineDensity.expanded);
    });
  });

  group('TurnStatusSnapshot and PendingMessage', () {
    test('copyWith clearToolName clears the tool name', () {
      final s = TurnStatusSnapshot(
        runId: 'r',
        lastEventKind: TurnEventKind.status,
        lastEventAt: DateTime(2026),
        currentToolName: 'web',
      );
      expect(s.copyWith(clearToolName: true).currentToolName, isNull);
      expect(s.copyWith(runId: 'r2').currentToolName, 'web');
    });

    test('PendingMessage defaults to no attachments', () {
      const p = PendingMessage(text: 't', conversationId: 'c');
      expect(p.attachmentIds, isEmpty);
      expect(p.attachments, isEmpty);
    });

    test('streamingPlaceholderId is stable', () {
      expect(streamingPlaceholderId, 'assistant-streaming');
    });
  });

  group('enum value sets', () {
    test('MessageStatus', () {
      expect(MessageStatus.values, [
        MessageStatus.sending,
        MessageStatus.ok,
        MessageStatus.failed,
        MessageStatus.stopped,
      ]);
    });

    test('ToolCallStatus', () {
      expect(ToolCallStatus.values, [
        ToolCallStatus.pending,
        ToolCallStatus.ok,
        ToolCallStatus.error,
        ToolCallStatus.denied,
        ToolCallStatus.cancelled,
      ]);
    });

    test('TimelineEntryType', () {
      expect(TimelineEntryType.values.map((e) => e.name), [
        'message',
        'thinking',
        'toolCall',
        'subagent',
        'command',
        'interaction',
      ]);
    });

    test('EntryState', () {
      expect(EntryState.values.map((e) => e.name), [
        'active',
        'complete',
        'stale',
        'awaitingInput',
      ]);
    });

    test('TurnEventKind', () {
      expect(TurnEventKind.values.map((e) => e.name), [
        'runStarted',
        'token',
        'status',
        'thinking',
        'toolResult',
        'subagentStarted',
        'subagentCompleted',
        'audioReady',
      ]);
    });
  });

  test('ChatAttachment is const-constructible', () {
    const a = ChatAttachment(
      id: '1',
      filename: 'f',
      mimeType: 'image/png',
      url: 'u',
    );
    expect(a.url, 'u');
  });
}
