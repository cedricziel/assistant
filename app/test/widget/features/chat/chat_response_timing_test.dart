import 'package:assistant_api/assistant_api.dart';
import 'package:assistant_app/api/models/server_profile.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/chat_screen.dart';
import 'package:assistant_app/features/connection/connection_provider.dart';
import 'package:assistant_app/features/personas/personas_provider.dart';
import 'package:assistant_app/features/settings/response_timing_preference.dart';
import 'package:assistant_app/shared/platform/adaptive_app.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();

  void push(ChatState s) => state = AsyncData(s);

  final loadedConversations = <String>[];

  @override
  Future<void> loadConversation(String conversationId) async =>
      loadedConversations.add(conversationId);
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

class _FakeShowResponseTimingNotifier extends ShowResponseTimingNotifier {
  _FakeShowResponseTimingNotifier(this.initial);

  bool initial;

  @override
  Future<bool> build() async => initial;

  void emit(bool value) => state = AsyncData(value);
}

const _timing = MessageTiming(
  timeToFirstToken: Duration(milliseconds: 800),
  total: Duration(milliseconds: 3000),
  outputTokens: 120,
  decode: Duration(milliseconds: 2200),
);

/// Two finished replies with timing: one live, one mapped from history.
ChatState _conversation() {
  final history = chatMessagesFromHistory([
    MessageSummary(
      (b) => b
        ..id = 'a2'
        ..role = 'assistant'
        ..content = 'From history'
        ..turn = 1
        ..createdAt = DateTime(2026, 1, 1)
        ..ttsAvailable = false
        ..timing.replace(
          MessageTimingSummary(
            (t) => t
              ..totalMs = 3000
              ..timeToFirstTokenMs = 800
              ..outputTokens = 120
              ..decodeMs = 2200,
          ),
        ),
    ),
  ]);

  return ChatState(
    conversationId: 'c1',
    messages: [
      ChatMessage(id: 'u1', role: 'user', content: 'hello'),
      ChatMessage(
        id: 'a1',
        role: 'assistant',
        content: 'Live reply',
        timing: _timing,
      ),
      ...history,
    ],
  );
}

Future<({_FakeChatNotifier chat, _FakeShowResponseTimingNotifier pref})> _pump(
  WidgetTester tester, {
  bool showResponseTiming = false,
}) async {
  final chat = _FakeChatNotifier();
  final pref = _FakeShowResponseTimingNotifier(showResponseTiming);
  tester.view.physicalSize = const Size(480, 1200);
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
        showResponseTimingProvider.overrideWith(() => pref),
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
  return (chat: chat, pref: pref);
}

void main() {
  group('ChatScreen response timing', () {
    testWidgets('off by default: no indicator while the model keeps timing', (
      tester,
    ) async {
      final app = await _pump(tester);
      app.chat.push(_conversation());
      await tester.pump();

      expect(find.byType(MessageTimingIndicator), findsNothing);
      final replies = app.chat.state.value!.messages
          .where((m) => !m.isUser)
          .toList();
      expect(replies, hasLength(2));
      expect(replies[0].timing?.total, const Duration(milliseconds: 3000));
      expect(
        replies[1].timing?.total,
        const Duration(milliseconds: 3000),
        reason: 'history-loaded replies keep their timing with the pref off',
      );
    });

    testWidgets(
      'turning the preference on shows every reply already on screen',
      (tester) async {
        final app = await _pump(tester);
        app.chat.push(_conversation());
        await tester.pump();
        expect(find.byType(MessageTimingIndicator), findsNothing);

        app.pref.emit(true);
        await tester.pump();

        expect(
          find.byType(MessageTimingIndicator),
          findsNWidgets(2),
          reason: 'already-rendered and history-loaded replies both appear',
        );
        expect(
          app.chat.loadedConversations,
          isEmpty,
          reason: 'the preference flip must not reload the conversation',
        );
      },
    );

    testWidgets('on from the start: finished replies show the indicator', (
      tester,
    ) async {
      final app = await _pump(tester, showResponseTiming: true);
      app.chat.push(_conversation());
      await tester.pump();

      expect(find.byType(MessageTimingIndicator), findsNWidgets(2));
    });

    testWidgets('a streaming reply shows no indicator with the preference on', (
      tester,
    ) async {
      final app = await _pump(tester, showResponseTiming: true);
      app.chat.push(
        ChatState(
          conversationId: 'c1',
          messages: [
            ChatMessage(id: 'u1', role: 'user', content: 'hello'),
            ChatMessage(
              id: 'a1',
              role: 'assistant',
              content: 'Still arriving',
              isStreaming: true,
              timing: _timing,
            ),
          ],
        ),
      );
      await tester.pump();

      expect(find.byType(MessageTimingIndicator), findsNothing);
    });
  });
}
