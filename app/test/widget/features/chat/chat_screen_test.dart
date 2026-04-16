import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/features/chat/audio_player_widget.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
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
  void prependConversation(ConversationSummary conv) {}
}

// ---------------------------------------------------------------------------
// Widget builder — wraps ChatScreen in MaterialApp with provider fakes.

Future<({_FakeChatNotifier notifier})> pumpChatScreen(
  WidgetTester tester, {
  ServerCapabilities? capabilities,
}) async {
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
        if (capabilities != null) ...[
          apiClientProvider.overrideWithValue(
            ApiClient(baseUrl: 'http://localhost', token: 'test'),
          ),
          capabilitiesProvider.overrideWith((ref) async => capabilities),
        ],
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

  group('Chat screen — scroll-to-bottom FAB', () {
    /// Builds a list of [count] messages to ensure the ListView has scroll
    /// extent in the 480×960 test viewport.
    List<ChatMessage> manyMessages(int count) => List.generate(
      count,
      (i) => ChatMessage(
        id: 'msg-$i',
        role: i.isEven ? 'user' : 'assistant',
        content: 'Message number $i with enough text to take up space.',
      ),
    );

    testWidgets('FAB is hidden when at bottom (initial state)', (tester) async {
      final res = await pumpChatScreen(tester);

      res.notifier.push(
        ChatState(conversationId: 'c1', messages: manyMessages(30)),
      );
      await tester.pumpAndSettle();

      // At rest the user is scrolled to the bottom — FAB must not be visible.
      expect(
        find.byKey(const Key('scroll_to_bottom_button')),
        findsNothing,
        reason: 'FAB should be hidden when the list is at the bottom',
      );
    });

    testWidgets('FAB appears after scrolling up', (tester) async {
      final res = await pumpChatScreen(tester);

      res.notifier.push(
        ChatState(conversationId: 'c1', messages: manyMessages(30)),
      );
      await tester.pumpAndSettle();

      // Drag the list down (i.e., scroll content upward).
      await tester.drag(find.byType(ListView), const Offset(0, 600));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('scroll_to_bottom_button')),
        findsOneWidget,
        reason: 'FAB should appear when scrolled away from bottom',
      );
    });

    testWidgets('tapping FAB scrolls back to bottom and hides FAB', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      res.notifier.push(
        ChatState(conversationId: 'c1', messages: manyMessages(30)),
      );
      await tester.pumpAndSettle();

      // Scroll up to reveal FAB.
      await tester.drag(find.byType(ListView), const Offset(0, 600));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('scroll_to_bottom_button')), findsOneWidget);

      // Tap the FAB.
      await tester.tap(find.byKey(const Key('scroll_to_bottom_button')));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('scroll_to_bottom_button')),
        findsNothing,
        reason: 'FAB should disappear after scrolling back to bottom',
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

  group('Chat screen — audio button visibility', () {
    testWidgets(
      'audio button hidden when ttsAvailable is false even with voiceReceive',
      (tester) async {
        final res = await pumpChatScreen(
          tester,
          capabilities: const ServerCapabilities(
            voiceSend: false,
            voiceReceive: true,
          ),
        );

        final msg = ChatMessage(
          id: 'a1',
          role: 'assistant',
          content: 'Hello there',
          ttsAvailable: false,
        );

        res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
        await tester.pump();

        expect(
          find.byType(AudioPlayerWidget),
          findsNothing,
          reason: 'Audio button must not appear when ttsAvailable is false',
        );
      },
    );

    testWidgets(
      'audio button shown when ttsAvailable is true and voiceReceive enabled',
      (tester) async {
        final res = await pumpChatScreen(
          tester,
          capabilities: const ServerCapabilities(
            voiceSend: false,
            voiceReceive: true,
          ),
        );

        final msg = ChatMessage(
          id: 'a2',
          role: 'assistant',
          content: 'Hello there',
          ttsAvailable: true,
        );

        res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
        await tester.pump();

        expect(
          find.byType(AudioPlayerWidget),
          findsOneWidget,
          reason:
              'Audio button should appear when ttsAvailable and voiceReceive are both true',
        );
      },
    );

    testWidgets(
      'audio button hidden when voiceReceive is false even with ttsAvailable',
      (tester) async {
        final res = await pumpChatScreen(
          tester,
          capabilities: const ServerCapabilities(
            voiceSend: false,
            voiceReceive: false,
          ),
        );

        final msg = ChatMessage(
          id: 'a3',
          role: 'assistant',
          content: 'Hello there',
          ttsAvailable: true,
        );

        res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
        await tester.pump();

        expect(
          find.byType(AudioPlayerWidget),
          findsNothing,
          reason: 'Audio button must not appear when voiceReceive is disabled',
        );
      },
    );
  });
}
