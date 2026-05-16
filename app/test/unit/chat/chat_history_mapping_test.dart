import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:built_collection/built_collection.dart';
import 'package:flutter_test/flutter_test.dart';

MessageSummary _msg({
  required String id,
  required String role,
  required String content,
  int turn = 0,
  List<({String name, String status, String? result})> toolCalls = const [],
  List<({String id, String filename, String mimeType, String url})>
      attachments =
      const [],
}) {
  return MessageSummary(
    (b) => b
      ..id = id
      ..role = role
      ..content = content
      ..turn = turn
      ..createdAt = DateTime.parse('2026-05-16T00:00:00Z')
      ..ttsAvailable = false
      ..toolCalls = toolCalls.isEmpty
          ? null
          : ListBuilder<ToolCallSummary>(
              toolCalls.map(
                (tc) => ToolCallSummary(
                  (t) => t
                    ..name = tc.name
                    ..status = tc.status
                    ..result = tc.result,
                ),
              ),
            )
      ..attachments = attachments.isEmpty
          ? null
          : ListBuilder<AttachmentMetaResponse>(
              attachments.map(
                (a) => AttachmentMetaResponse(
                  (x) => x
                    ..id = a.id
                    ..filename = a.filename
                    ..mimeType = a.mimeType
                    ..url = a.url
                    ..sizeBytes = 1
                    ..createdAt = DateTime.parse('2026-05-16T00:00:00Z'),
                ),
              ),
            ),
  );
}

void main() {
  group('chatMessagesFromHistory', () {
    test(
      'assistant tool-call-only turns render as chips, not empty bubbles',
      () {
        // Mirrors the actual schorschvm conversation shape: 3 ReAct iterations
        // each persisted as `assistant(content="", tool_calls=[X])` followed
        // by a `tool` result row, terminated by an assistant final answer.
        final history = [
          _msg(id: 'u1', role: 'user', content: 'Hello', turn: 0),
          _msg(
            id: 'a1',
            role: 'assistant',
            content: '',
            turn: 1,
            toolCalls: [(name: 'bash', status: 'ok', result: 'ok')],
          ),
          _msg(
            id: 'a2',
            role: 'assistant',
            content: '',
            turn: 2,
            toolCalls: [(name: 'web-fetch', status: 'ok', result: 'ok')],
          ),
          _msg(
            id: 'a3',
            role: 'assistant',
            content: '',
            turn: 3,
            toolCalls: [(name: 'slack-post', status: 'ok', result: 'ok')],
          ),
          _msg(id: 'a4', role: 'assistant', content: 'Done.', turn: 4),
        ];

        final mapped = chatMessagesFromHistory(history);

        // For each tool-only assistant row we expect exactly one toolCall
        // timeline entry and *no* accompanying message bubble.
        final toolCallEntries = mapped
            .where((m) => m.timelineType == TimelineEntryType.toolCall)
            .toList();
        expect(toolCallEntries, hasLength(3));
        expect(
          toolCallEntries.map((m) => m.toolCalls.single.toolName).toList(),
          ['bash', 'web-fetch', 'slack-post'],
        );

        // No empty assistant message bubbles. The user row, three chips,
        // and the final-answer row are the only entries we expect.
        final messageBubbles = mapped
            .where((m) => m.timelineType == TimelineEntryType.message)
            .toList();
        expect(messageBubbles, hasLength(2));
        expect(messageBubbles[0].role, 'user');
        expect(messageBubbles[0].content, 'Hello');
        expect(messageBubbles[1].role, 'assistant');
        expect(messageBubbles[1].content, 'Done.');

        for (final m in messageBubbles) {
          expect(
            m.content.isEmpty && m.role == 'assistant',
            isFalse,
            reason: 'Empty assistant content bubble leaked through',
          );
        }
      },
    );

    test(
      'assistant message with both content and tool calls keeps the bubble',
      () {
        final history = [
          _msg(
            id: 'a1',
            role: 'assistant',
            content: 'Calling tool first.',
            toolCalls: [(name: 'bash', status: 'ok', result: 'ok')],
          ),
        ];

        final mapped = chatMessagesFromHistory(history);

        expect(
          mapped.where((m) => m.timelineType == TimelineEntryType.toolCall),
          hasLength(1),
        );
        final bubbles = mapped
            .where((m) => m.timelineType == TimelineEntryType.message)
            .toList();
        expect(bubbles, hasLength(1));
        expect(bubbles.single.content, 'Calling tool first.');
      },
    );

    test('user messages are always preserved as bubbles', () {
      final history = [
        _msg(id: 'u1', role: 'user', content: 'Hi'),
        _msg(id: 'u2', role: 'user', content: ''),
      ];

      final mapped = chatMessagesFromHistory(history);
      final bubbles = mapped
          .where((m) => m.timelineType == TimelineEntryType.message)
          .toList();
      expect(bubbles, hasLength(2));
      expect(bubbles.every((m) => m.role == 'user'), isTrue);
    });

    test('toolCall chip entries carry name, status, and result', () {
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'bash', status: 'ok', result: 'stdout: ok'),
            (name: 'web-fetch', status: 'error', result: '404'),
          ],
        ),
      ];

      final mapped = chatMessagesFromHistory(history);
      final chips = mapped
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .toList();
      expect(chips, hasLength(2));
      expect(chips[0].toolCalls.single.toolName, 'bash');
      expect(chips[0].toolCalls.single.status, ToolCallStatus.ok);
      expect(chips[0].toolCalls.single.result, 'stdout: ok');
      expect(chips[1].toolCalls.single.toolName, 'web-fetch');
      expect(chips[1].toolCalls.single.status, ToolCallStatus.error);
      expect(chips[1].toolCalls.single.result, '404');
    });

    test(
      'tool-only assistant row WITH attachments keeps the bubble for thumbnails',
      () {
        // Edge case: the model invoked a tool AND produced an image attachment
        // but no reply text. The bubble must remain so the attachment renders.
        final history = [
          _msg(
            id: 'a1',
            role: 'assistant',
            content: '',
            toolCalls: [(name: 'image-gen', status: 'ok', result: 'ok')],
            attachments: [
              (
                id: 'att1',
                filename: 'render.png',
                mimeType: 'image/png',
                url: 'https://example/render.png',
              ),
            ],
          ),
        ];

        final mapped = chatMessagesFromHistory(history);
        final bubbles = mapped
            .where((m) => m.timelineType == TimelineEntryType.message)
            .toList();
        expect(bubbles, hasLength(1));
        expect(bubbles.single.role, 'assistant');
        expect(bubbles.single.content, '');
        expect(bubbles.single.attachments, hasLength(1));
        expect(bubbles.single.attachments.single.filename, 'render.png');
        expect(
          mapped.where((m) => m.timelineType == TimelineEntryType.toolCall),
          hasLength(1),
        );
      },
    );

    test('pending tool-call status round-trips (not coerced to ok)', () {
      // A persisted row could carry a non-terminal status if the orchestrator
      // crashed mid-call. The mapper must surface it as `pending` instead of
      // silently treating it as a successful invocation.
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [(name: 'bash', status: 'pending', result: null)],
        ),
      ];

      final chips = chatMessagesFromHistory(
        history,
      ).where((m) => m.timelineType == TimelineEntryType.toolCall).toList();
      expect(chips, hasLength(1));
      expect(chips.single.toolCalls.single.status, ToolCallStatus.pending);
    });

    test('multiple invocations of the same tool produce distinct chip ids', () {
      // Defensive: if a single assistant row carries two `bash` invocations,
      // both chips must have unique ids so Flutter widget keying does not
      // collide.
      final history = [
        _msg(
          id: 'a1',
          role: 'assistant',
          content: '',
          toolCalls: [
            (name: 'bash', status: 'ok', result: 'first'),
            (name: 'bash', status: 'ok', result: 'second'),
          ],
        ),
      ];

      final ids = chatMessagesFromHistory(history)
          .where((m) => m.timelineType == TimelineEntryType.toolCall)
          .map((m) => m.id)
          .toList();
      expect(ids, hasLength(2));
      expect(ids.toSet(), hasLength(2), reason: 'chip ids must be unique');
    });
  });
}
