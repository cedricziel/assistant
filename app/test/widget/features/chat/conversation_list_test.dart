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
          builder: (_, _) => const Scaffold(body: ConversationList()),
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
}
