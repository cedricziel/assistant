import 'package:assistant_app/features/chat/chat_provider.dart';
import 'package:assistant_app/features/chat/conversation_list.dart';
import 'package:assistant_app/features/chat/conversation_search_provider.dart';
import 'package:assistant_ui/assistant_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:material_ui/material_ui.dart';

class _FakeListNotifier extends ConversationListNotifier {
  @override
  Future<ConversationListState> build() async => const ConversationListState();
}

class _FakeChatNotifier extends ChatNotifier {
  @override
  Future<ChatState> build() async => const ChatState();
}

/// Serves [next] as the search state and counts retries.
class _FakeSearchNotifier extends ConversationSearchNotifier {
  _FakeSearchNotifier(this.next);

  ConversationSearchState next;
  int retries = 0;

  @override
  ConversationSearchState build() {
    ref.listen(conversationSearchQueryProvider, (_, query) {
      state = query.trim().isEmpty ? const ConversationSearchIdle() : next;
    });
    return const ConversationSearchIdle();
  }

  @override
  void retry() => retries++;
}

const _item = ConversationSearchItem(
  id: 'a',
  title: 'Lisbon trip',
  titleSegments: [
    SearchSegmentItem(text: 'Lisbon', highlighted: true),
    SearchSegmentItem(text: ' trip', highlighted: false),
  ],
);

Future<_FakeSearchNotifier> _pump(
  WidgetTester tester, {
  required ConversationSearchState result,
  VoidCallback? onSelected,
}) async {
  final search = _FakeSearchNotifier(result);
  final router = GoRouter(
    routes: [
      GoRoute(
        path: '/',
        builder: (_, _) => Scaffold(
          body: ConversationListPane(onConversationSelected: onSelected),
        ),
      ),
      GoRoute(
        path: '/chat/:id',
        builder: (_, s) =>
            Scaffold(body: Text('Chat ${s.pathParameters['id']}')),
      ),
    ],
  );
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        conversationListProvider.overrideWith(_FakeListNotifier.new),
        chatProvider.overrideWith(_FakeChatNotifier.new),
        conversationSearchProvider.overrideWith(() => search),
      ],
      child: MaterialApp.router(routerConfig: router),
    ),
  );
  await tester.pumpAndSettle();
  return search;
}

/// A focused text field blinks its caret forever, so `pumpAndSettle` never
/// returns while the search field has focus.
Future<void> _settle(WidgetTester tester) async {
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
}

final _field = find.byKey(const Key('conversation_search_field'));

void main() {
  testWidgets('typing swaps the list for the search results', (tester) async {
    await _pump(tester, result: const ConversationSearchData([_item]));
    expect(find.byType(ConversationSearchResultsView), findsNothing);

    await tester.enterText(_field, 'lisb');
    await _settle(tester);

    expect(find.byType(ConversationSearchResultsView), findsOneWidget);
    expect(
      find.byKey(const Key('conversation_search_result_a')),
      findsOneWidget,
    );
    expect(find.byKey(const Key('new_chat_button')), findsOneWidget);
  });

  testWidgets('no matches shows the empty message', (tester) async {
    await _pump(tester, result: const ConversationSearchData([]));

    await tester.enterText(_field, 'zzz');
    await _settle(tester);

    expect(find.text('No conversations match'), findsOneWidget);
  });

  testWidgets('an error offers Retry', (tester) async {
    final search = await _pump(
      tester,
      result: const ConversationSearchError('boom'),
    );

    await tester.enterText(_field, 'lisb');
    await _settle(tester);
    await tester.tap(find.text('Retry'));

    expect(search.retries, 1);
  });

  testWidgets('tapping a result navigates, clears the query and closes', (
    tester,
  ) async {
    var selected = 0;
    await _pump(
      tester,
      result: const ConversationSearchData([_item]),
      onSelected: () => selected++,
    );

    await tester.enterText(_field, 'lisb');
    await _settle(tester);
    await tester.tap(find.byKey(const Key('conversation_search_result_a')));
    await _settle(tester);

    expect(find.text('Chat a'), findsOneWidget);
    expect(selected, 1);
    final container = ProviderScope.containerOf(
      tester.element(find.text('Chat a')),
    );
    expect(container.read(conversationSearchQueryProvider), '');
  });

  testWidgets('the clear button restores the list', (tester) async {
    await _pump(tester, result: const ConversationSearchData([_item]));
    await tester.enterText(_field, 'lisb');
    await _settle(tester);

    await tester.tap(find.byKey(const Key('conversation_search_clear')));
    await _settle(tester);

    expect(find.byType(ConversationSearchResultsView), findsNothing);
    expect(find.byType(ConversationListView), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      '',
    );
  });

  testWidgets('Esc clears the query and restores the list', (tester) async {
    await _pump(tester, result: const ConversationSearchData([_item]));
    await tester.enterText(_field, 'lisb');
    await _settle(tester);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await _settle(tester);

    expect(find.byType(ConversationSearchResultsView), findsNothing);
    expect(find.byType(ConversationListView), findsOneWidget);
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller?.text,
      '',
    );
  });

  testWidgets('the field keeps focus while results replace the list', (
    tester,
  ) async {
    await _pump(tester, result: const ConversationSearchData([_item]));

    await tester.enterText(_field, 'l');
    await _settle(tester);
    await tester.enterText(_field, 'li');
    await _settle(tester);

    expect(
      tester.widget<EditableText>(find.byType(EditableText)).focusNode.hasFocus,
      isTrue,
    );
  });
}
