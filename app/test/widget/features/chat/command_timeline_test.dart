import 'package:assistant_api/assistant_api.dart' hide ServerCapabilities;
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/chat/command_event_tile.dart';
import 'package:assistant_app/features/chat/commands_provider.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

// -- Fake notifiers -----------------------------------------------------------

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

// -- Tests --------------------------------------------------------------------

void main() {
  group('Command events in timeline', () {
    testWidgets('timeline renders CommandEventTile for command entries', (
      tester,
    ) async {
      final chatNotifier = _FakeChatNotifier();

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
            apiClientProvider.overrideWithValue(null),
            commandsProvider.overrideWith((ref) async => []),
          ],
          child: const MaterialApp(home: ChatScreen()),
        ),
      );
      await tester.pumpAndSettle();

      // Push a state with mixed messages and a command event.
      chatNotifier.push(
        ChatState(
          conversationId: 'conv-1',
          messages: [
            ChatMessage(id: 'msg-1', role: 'user', content: 'Hello'),
            ChatMessage(
              id: 'cmd-1',
              role: 'system',
              content: 'Model set to gpt-4o',
              timelineType: TimelineEntryType.command,
              commandName: 'model',
              commandAckText: 'Model set to gpt-4o',
            ),
            ChatMessage(id: 'msg-2', role: 'assistant', content: 'Hi there!'),
          ],
        ),
      );
      await tester.pumpAndSettle();

      // The command event should render as a CommandEventTile.
      expect(find.byType(CommandEventTile), findsOneWidget);
      expect(find.text('/model'), findsOneWidget);
      expect(find.text('Model set to gpt-4o'), findsOneWidget);
    });

    testWidgets('multiple command events render in order', (tester) async {
      final chatNotifier = _FakeChatNotifier();

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
            apiClientProvider.overrideWithValue(null),
            commandsProvider.overrideWith((ref) async => []),
          ],
          child: const MaterialApp(home: ChatScreen()),
        ),
      );
      await tester.pumpAndSettle();

      chatNotifier.push(
        ChatState(
          conversationId: 'conv-2',
          messages: [
            ChatMessage(
              id: 'cmd-1',
              role: 'system',
              content: 'New conversation started',
              timelineType: TimelineEntryType.command,
              commandName: 'new',
              commandAckText: 'New conversation started',
            ),
            ChatMessage(id: 'msg-1', role: 'user', content: 'Hello'),
            ChatMessage(
              id: 'cmd-2',
              role: 'system',
              content: 'History compacted',
              timelineType: TimelineEntryType.command,
              commandName: 'compact',
              commandAckText: 'History compacted',
            ),
          ],
        ),
      );
      await tester.pumpAndSettle();

      // Both command events should render.
      expect(find.byType(CommandEventTile), findsNWidgets(2));
      expect(find.text('/new'), findsOneWidget);
      expect(find.text('/compact'), findsOneWidget);
    });
  });
}
