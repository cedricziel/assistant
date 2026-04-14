import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// ---------------------------------------------------------------------------
// Fake notifiers — let tests push arbitrary state directly.

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);
}

class _FakePersonasNotifier extends PersonasNotifier {
  @override
  Future<PersonasState> build() async => const PersonasState();
}

class _FakeConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<void> refresh() async {}

  @override
  void prependConversation(conv) {}
}

// ---------------------------------------------------------------------------
// Widget builder — wraps ChatScreen in MaterialApp with provider fakes.

Future<({_FakeChatNotifier notifier})> pumpChatScreen(
  WidgetTester tester,
) async {
  final chatNotifier = _FakeChatNotifier();

  // Use a narrow portrait viewport so no sidebar is shown (isWide = false).
  tester.view.physicalSize = const Size(480, 960);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        chatProvider.overrideWith(() => chatNotifier),
        personasProvider.overrideWith(() => _FakePersonasNotifier()),
        conversationListProvider.overrideWith(
          () => _FakeConversationListNotifier(),
        ),
      ],
      child: const MaterialApp(home: ChatScreen()),
    ),
  );

  // Let async providers settle.
  await tester.pumpAndSettle();
  return (notifier: chatNotifier);
}

// ---------------------------------------------------------------------------

void main() {
  group('Chat screen — input behaviour', () {
    testWidgets('7.5: message_input field is enabled while isSending == true', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      // Push isSending = true (streaming in progress).
      res.notifier.push(
        const ChatState(
          conversationId: 'c1',
          isSending: true,
          streamingContent: '',
        ),
      );
      await tester.pump();

      final textField = tester.widget<TextField>(
        find.byKey(const Key('message_input')),
      );
      expect(
        textField.enabled,
        isNot(false),
        reason: 'input field must not be disabled while isSending',
      );
    });
  });

  group('Chat screen — retry affordance', () {
    testWidgets(
      '7.6: Retry button appears when a user message has status == failed',
      (tester) async {
        final res = await pumpChatScreen(tester);

        final failedMsg = ChatMessage(
          id: 'user-1',
          role: 'user',
          content: 'send me again',
          status: MessageStatus.failed,
        );

        res.notifier.push(
          ChatState(conversationId: 'c1', messages: [failedMsg]),
        );
        await tester.pump();

        expect(
          find.byKey(const Key('retry_button')),
          findsOneWidget,
          reason: 'Retry button should appear for a failed message',
        );
      },
    );

    testWidgets('Retry button is absent when user message has status == ok', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      final okMsg = ChatMessage(
        id: 'user-2',
        role: 'user',
        content: 'hello',
        status: MessageStatus.ok,
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [okMsg]));
      await tester.pump();

      expect(find.byKey(const Key('retry_button')), findsNothing);
    });
  });
}
