import 'package:assistant_app/api/models/stream_event.dart';
import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/conversation_list.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

/// Records the `title` argument of every [createConversation] call.
class _FakeConversationListNotifier extends ConversationListNotifier {
  final List<String?> titles = [];

  @override
  Future<ConversationListState> build() async => const ConversationListState();

  @override
  Future<String?> createConversation({String? title}) async {
    titles.add(title);
    return 'new-convo-123';
  }
}

class _SeededConversationListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => ConversationListState(
    conversations: [
      ConversationListEntry(
        id: 'p1',
        title: 'Pinned chat',
        createdAt: DateTime.utc(2024, 1, 1),
        updatedAt: DateTime.utc(2024, 1, 1),
        pinnedAt: DateTime.utc(2024, 2, 1),
      ),
      ConversationListEntry(
        id: 'r1',
        title: 'Recent chat',
        createdAt: DateTime.utc(2024, 1, 1),
        updatedAt: DateTime.utc(2024, 3, 1),
      ),
    ],
  );
}

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();
}

void main() {
  testWidgets('New Chat button creates a conversation without a title', (
    tester,
  ) async {
    final notifier = _FakeConversationListNotifier();
    final router = GoRouter(
      routes: [
        GoRoute(
          path: '/',
          builder: (_, _) => const Scaffold(body: ConversationListPane()),
        ),
        GoRoute(
          path: '/chat/:id',
          builder: (_, _) => const Scaffold(body: Text('Chat')),
        ),
      ],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          conversationListProvider.overrideWith(() => notifier),
          chatProvider.overrideWith(_FakeChatNotifier.new),
        ],
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('new_chat_button')));
    await tester.pumpAndSettle();

    expect(notifier.titles, [
      null,
    ], reason: 'An explicit title would lock the row against auto-titling');
  });

  testWidgets('shows a Pinned group and highlights the selected conversation', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          conversationListProvider.overrideWith(
            _SeededConversationListNotifier.new,
          ),
          chatProvider.overrideWith(_FakeChatNotifier.new),
        ],
        child: const MaterialApp(
          home: Scaffold(body: ConversationListPane(selectedId: 'r1')),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Pinned'), findsOneWidget);
    final tiles = tester.widgetList<ListTile>(find.byType(ListTile)).toList();
    expect(tiles.map((t) => t.selected), [false, true]);
  });
}
