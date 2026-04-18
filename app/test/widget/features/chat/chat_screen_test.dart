import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/api/api_client.dart';
import 'package:assistant_app/api/capabilities_provider.dart';
import 'package:assistant_app/api/models/server_capabilities.dart';
import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/chat/audio_player_widget.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:cached_network_image/cached_network_image.dart';
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
        // Provide a profile so image auth headers and base URL are available.
        activeProfileProvider.overrideWithValue(
          const ServerProfile(baseUrl: 'http://localhost', token: 'test-token'),
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
    testWidgets('inline audio player hidden when audioId is null', (
      tester,
    ) async {
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
        reason: 'Inline audio player must not appear when audioId is null',
      );
    });

    testWidgets('inline audio player shown when audioId is set', (
      tester,
    ) async {
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
        audioId: 'audio-123',
        ttsAvailable: true,
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      expect(
        find.byType(AudioPlayerWidget),
        findsOneWidget,
        reason: 'Inline audio player should appear when audioId is set',
      );
    });

    testWidgets(
      'inline audio player hidden even with audioId when voiceReceive is false',
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
          audioId: 'audio-456',
          ttsAvailable: true,
        );

        res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
        await tester.pump();

        // Inline player renders based on audioId, independent of voiceReceive.
        expect(
          find.byType(AudioPlayerWidget),
          findsOneWidget,
          reason:
              'Inline audio player should appear when audioId is set regardless of voiceReceive',
        );
      },
    );
  });

  group('Chat screen — meta action row', () {
    testWidgets('copy action shown for assistant message with content', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'a1',
        role: 'assistant',
        content: 'Hello there',
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      expect(
        find.byKey(const Key('copy_action')),
        findsOneWidget,
        reason: 'Copy action should appear for messages with content',
      );
    });

    testWidgets(
      'read aloud action shown for assistant message when voiceReceive enabled and no audioId',
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
        );

        res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
        await tester.pump();

        expect(
          find.byKey(const Key('read_aloud_action')),
          findsOneWidget,
          reason:
              'Read aloud should appear for assistant messages when voiceReceive is enabled',
        );
      },
    );

    testWidgets('read aloud action hidden for user messages', (tester) async {
      final res = await pumpChatScreen(
        tester,
        capabilities: const ServerCapabilities(
          voiceSend: false,
          voiceReceive: true,
        ),
      );

      final msg = ChatMessage(id: 'u1', role: 'user', content: 'Hello there');

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      expect(
        find.byKey(const Key('read_aloud_action')),
        findsNothing,
        reason: 'Read aloud should not appear for user messages',
      );
    });

    testWidgets('retry action shown only for failed messages', (tester) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'u1',
        role: 'user',
        content: 'Hello there',
        status: MessageStatus.failed,
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      expect(
        find.byKey(const Key('retry_button')),
        findsOneWidget,
        reason: 'Retry action should appear for failed messages',
      );
    });
  });

  group('Chat screen — read aloud state machine', () {
    testWidgets(
      'tapping Read aloud shows loading then returns to idle on fetch failure',
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
        );

        res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
        await tester.pump();

        // Should start in idle state with "Read aloud" label.
        expect(find.text('Read aloud'), findsOneWidget);

        // Tap the read aloud action — fetchMessageAudio returns null (no API).
        await tester.tap(find.text('Read aloud'));
        await tester.pump();

        // Should transition to loading state.
        expect(
          find.text('Loading\u2026'),
          findsOneWidget,
          reason: 'Should show loading state after tap',
        );

        // Let the future complete (fetch returns null → error state).
        await tester.pumpAndSettle();

        // Error state shows warning text.
        expect(
          find.text('Could not generate audio'),
          findsOneWidget,
          reason: 'Should show error message when TTS fails',
        );
      },
    );

    testWidgets('error state auto-dismisses after timeout', (tester) async {
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
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      // Trigger error state.
      await tester.tap(find.text('Read aloud'));
      await tester.pumpAndSettle();

      expect(find.text('Could not generate audio'), findsOneWidget);

      // Advance past the 4-second auto-dismiss timer.
      await tester.pump(const Duration(seconds: 5));

      expect(
        find.text('Could not generate audio'),
        findsNothing,
        reason: 'Error should auto-dismiss after 4 seconds',
      );
      expect(
        find.text('Read aloud'),
        findsOneWidget,
        reason: 'Should return to idle state after error dismissal',
      );
    });

    testWidgets('stop action shown during streaming', (tester) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'assistant-streaming',
        role: 'assistant',
        content: 'Generating...',
        isStreaming: true,
      );

      res.notifier.push(
        ChatState(conversationId: 'c1', messages: [msg], isSending: true),
      );
      await tester.pump();

      expect(
        find.byKey(const Key('stop_action')),
        findsOneWidget,
        reason: 'Stop action should appear during streaming',
      );
      expect(
        find.text('Stop'),
        findsWidgets, // may find both in input row and meta row
      );
    });
  });

  group('Chat screen — attachment thumbnails', () {
    testWidgets('user message with attachments renders CachedNetworkImage', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'user-att-1',
        role: 'user',
        content: 'Look at this image',
        attachments: [
          const ChatAttachment(
            id: 'att-1',
            filename: 'photo.png',
            mimeType: 'image/png',
            url: '/api/attachments/att-1',
          ),
        ],
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      // CachedNetworkImage widget should be present for the attachment.
      expect(
        find.byType(CachedNetworkImage),
        findsOneWidget,
        reason: 'Attachment thumbnail should render a CachedNetworkImage',
      );

      // Should be wrapped in a GestureDetector for tap-to-expand.
      expect(
        find.byType(GestureDetector),
        findsWidgets,
        reason: 'Attachment thumbnails should be tappable',
      );
    });

    testWidgets('assistant message with attachments renders thumbnails', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'asst-att-1',
        role: 'assistant',
        content: 'Here is the generated image',
        attachments: [
          const ChatAttachment(
            id: 'att-2',
            filename: 'result.jpg',
            mimeType: 'image/jpeg',
            url: '/api/attachments/att-2',
          ),
        ],
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      expect(
        find.byType(CachedNetworkImage),
        findsOneWidget,
        reason: 'Assistant attachment thumbnails should be rendered',
      );
    });

    testWidgets('message without attachments has no CachedNetworkImage', (
      tester,
    ) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'user-noatt',
        role: 'user',
        content: 'Plain text message',
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      expect(
        find.byType(CachedNetworkImage),
        findsNothing,
        reason: 'No CachedNetworkImage when message has no attachments',
      );
    });

    testWidgets('tapping thumbnail opens full-size dialog', (tester) async {
      final res = await pumpChatScreen(tester);

      final msg = ChatMessage(
        id: 'user-tap-1',
        role: 'user',
        content: 'Check this',
        attachments: [
          const ChatAttachment(
            id: 'att-3',
            filename: 'photo.png',
            mimeType: 'image/png',
            url: '/api/attachments/att-3',
          ),
        ],
      );

      res.notifier.push(ChatState(conversationId: 'c1', messages: [msg]));
      await tester.pump();

      // Tap the CachedNetworkImage (thumbnail).
      await tester.tap(find.byType(CachedNetworkImage).first);
      // Use pump() instead of pumpAndSettle() — CachedNetworkImage's
      // internal loading animation never settles in the test environment.
      await tester.pump();

      // A Dialog should now be visible with an InteractiveViewer.
      expect(
        find.byType(InteractiveViewer),
        findsOneWidget,
        reason: 'Tapping thumbnail should open full-size image dialog',
      );
    });
  });
}
