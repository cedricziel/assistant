import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _FakeChatNotifier extends ChatNotifier {
  final List<String> sent = [];

  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);

  @override
  Future<void> sendMessage(
    String message, {
    List<String> attachmentIds = const [],
    List<ChatAttachment> attachments = const [],
  }) async => sent.add(message);
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
}

Future<_FakeChatNotifier> _pump(WidgetTester tester) async {
  final chat = _FakeChatNotifier();
  tester.view.physicalSize = const Size(480, 960);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        chatProvider.overrideWith(() => chat),
        personasProvider.overrideWith(_FakePersonasNotifier.new),
        conversationListProvider.overrideWith(
          _FakeConversationListNotifier.new,
        ),
        activeProfileProvider.overrideWithValue(
          const ServerProfile(baseUrl: 'http://localhost', token: 'test-token'),
        ),
      ],
      child: MaterialApp(
        home: const ChatScreen(),
        builder: (_, child) => LegacyDesignBridge(child: child!),
      ),
    ),
  );
  await tester.pumpAndSettle();
  return chat;
}

ChatState _state({
  List<String> suggestions = const [],
  bool isSending = false,
  bool isReconnecting = false,
}) => ChatState(
  conversationId: 'c1',
  messages: [
    ChatMessage(id: 'm1', role: 'user', content: 'hello'),
    ChatMessage(id: 'm2', role: 'assistant', content: 'hi'),
  ],
  suggestions: suggestions,
  isSending: isSending,
  isReconnecting: isReconnecting,
);

void main() {
  group('ChatScreen follow-up suggestions', () {
    testWidgets('chip sits above the composer and a tap sends the prompt', (
      tester,
    ) async {
      final chat = await _pump(tester);
      chat.push(_state(suggestions: ['Show an example', 'Go deeper']));
      await tester.pump();

      final chip = find.byKey(const Key('suggestion_chip_0'));
      expect(chip, findsOneWidget);
      expect(find.byKey(const Key('suggestion_chip_1')), findsOneWidget);
      expect(
        tester.getBottomLeft(chip).dy,
        lessThan(tester.getTopLeft(find.byType(Composer)).dy),
        reason: 'chips render above the composer',
      );

      await tester.tap(chip);
      await tester.pump();

      expect(chat.sent, ['Show an example']);
    });

    testWidgets('no chips render while a turn is sending', (tester) async {
      final chat = await _pump(tester);
      chat.push(_state(suggestions: ['Show an example'], isSending: true));
      await tester.pump();

      expect(find.byKey(const Key('suggestion_chip_0')), findsNothing);
    });

    testWidgets('no chips render without suggestions', (tester) async {
      final chat = await _pump(tester);
      chat.push(_state());
      await tester.pump();

      expect(find.byKey(const Key('suggestion_chip_0')), findsNothing);
    });

    testWidgets('the reconnect banner stays above the chips', (tester) async {
      final chat = await _pump(tester);
      chat.push(_state(suggestions: ['Show an example'], isReconnecting: true));
      await tester.pump();

      final banner = find.text('Reconnecting…');
      expect(banner, findsOneWidget);
      expect(
        tester.getTopLeft(banner).dy,
        lessThan(
          tester.getTopLeft(find.byKey(const Key('suggestion_chip_0'))).dy,
        ),
      );
    });
  });
}
