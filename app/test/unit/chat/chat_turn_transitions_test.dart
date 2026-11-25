import 'package:assistant_app/features/chat/chat_models.dart';
import 'package:assistant_app/features/chat/chat_turn_transitions.dart';
import 'package:flutter_test/flutter_test.dart';

ChatMessage _user(String id) => ChatMessage(
  id: id,
  role: 'user',
  content: 'hi',
  status: MessageStatus.sending,
);

ChatMessage _placeholder() => ChatMessage(
  id: streamingPlaceholderId,
  role: 'assistant',
  content: 'partial',
  isStreaming: true,
);

ChatState _streaming() => ChatState(
  conversationId: 'c1',
  messages: [_user('u1'), _placeholder()],
  isSending: true,
  streamingContent: 'partial',
  statusMessage: 'Running tool: x',
  currentTurnStatus: TurnStatusSnapshot(
    runId: 'r1',
    lastEventKind: TurnEventKind.token,
    lastEventAt: DateTime(2026),
  ),
  pendingQueue: const [PendingMessage(text: 'next', conversationId: 'c1')],
);

void main() {
  group('abortedTurnState', () {
    test('drops the placeholder and stops sending', () {
      final next = abortedTurnState(_streaming());

      expect(next.messages.map((m) => m.id), ['u1']);
      expect(next.isSending, isFalse);
      expect(next.streamingContent, isEmpty);
      expect(next.error, isNull);
    });

    test('marks the named user message failed', () {
      final next = abortedTurnState(_streaming(), failedUserMsgId: 'u1');
      expect(next.messages.single.status, MessageStatus.failed);
    });

    test('leaves the user message status alone without an id', () {
      final next = abortedTurnState(_streaming());
      expect(next.messages.single.status, MessageStatus.sending);
    });

    test('surfaces the error and keeps conversation and queue', () {
      final next = abortedTurnState(_streaming(), error: 'boom');
      expect(next.error, 'boom');
      expect(next.conversationId, 'c1');
      expect(next.pendingQueue, hasLength(1));
    });

    test('keeps status message and turn status unless asked to clear them', () {
      final kept = abortedTurnState(_streaming());
      expect(kept.statusMessage, 'Running tool: x');
      expect(kept.currentTurnStatus, isNotNull);

      final cleared = abortedTurnState(
        _streaming(),
        clearStatusMessage: true,
        clearTurnStatus: true,
      );
      expect(cleared.statusMessage, isNull);
      expect(cleared.currentTurnStatus, isNull);
    });
  });

  group('completedTurnState', () {
    test('swaps the placeholder for the final message and resets the turn', () {
      final next = completedTurnState(
        _streaming(),
        conversationId: 'c1',
        userMsgId: 'u1',
        assistantId: 'a1',
        content: 'final',
      );

      expect(next.messages.map((m) => m.id), ['u1', 'a1']);
      expect(next.messages.first.status, MessageStatus.ok);
      expect(next.messages.last.content, 'final');
      expect(next.messages.last.isStreaming, isFalse);
      expect(next.isSending, isFalse);
      expect(next.streamingContent, isEmpty);
      expect(next.currentTurnStatus, isNull);
      expect(next.statusMessage, isNull);
    });

    test('keeps only the conversation id and the pending queue', () {
      final next = completedTurnState(
        _streaming(),
        conversationId: 'c9',
        userMsgId: 'u1',
        assistantId: 'a1',
        content: 'final',
      );
      expect(next.conversationId, 'c9');
      expect(next.pendingQueue, hasLength(1));
    });

    test('carries the audio id and tts flag over from the placeholder', () {
      final state = _streaming();
      state.messages[state.messages.length - 1] = state.messages.last.copyWith(
        audioId: 'audio-1',
        ttsAvailable: true,
      );

      final next = completedTurnState(
        state,
        conversationId: 'c1',
        userMsgId: 'u1',
        assistantId: 'a1',
        content: 'final',
      );

      expect(next.messages.last.audioId, 'audio-1');
      expect(next.messages.last.ttsAvailable, isTrue);
    });

    test('completes streaming thinking entries', () {
      final state = _streaming();
      state.messages.insert(
        1,
        ChatMessage(
          id: 'thinking-1',
          role: 'assistant',
          content: '',
          isStreaming: true,
          timelineType: TimelineEntryType.thinking,
          thinkingContent: 'hmm',
        ),
      );

      final next = completedTurnState(
        state,
        conversationId: 'c1',
        userMsgId: 'u1',
        assistantId: 'a1',
        content: 'final',
      );

      final thinking = next.messages.firstWhere(
        (m) => m.timelineType == TimelineEntryType.thinking,
      );
      expect(thinking.isStreaming, isFalse);
    });
  });
}
