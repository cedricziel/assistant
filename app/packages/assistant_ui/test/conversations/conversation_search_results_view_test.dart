import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

const _titleHit = ConversationSearchItem(
  id: 'c1',
  title: 'Lisbon trip',
  titleSegments: [
    SearchSegmentItem(text: 'Lisbon', highlighted: true),
    SearchSegmentItem(text: ' trip', highlighted: false),
  ],
);

final _messageHits = ConversationSearchItem(
  id: 'c2',
  title: 'Weekend plans',
  titleSegments: const [
    SearchSegmentItem(text: 'Weekend plans', highlighted: false),
  ],
  matches: [
    for (var i = 1; i <= 4; i++)
      [
        SearchSegmentItem(text: 'snippet $i with ', highlighted: false),
        const SearchSegmentItem(text: 'hotel', highlighted: true),
      ],
  ],
);

final _archived = ConversationSearchItem(
  id: 'c3',
  title: 'Old notes',
  titleSegments: const [
    SearchSegmentItem(text: 'Old notes', highlighted: false),
  ],
  archivedAt: DateTime.utc(2026, 1, 2),
);

Widget _host(Widget child) => MaterialApp(home: Scaffold(body: child));

void main() {
  testWidgets('renders the highlighted title of each result', (tester) async {
    await tester.pumpWidget(
      _host(
        ConversationSearchResultsView(
          results: [_titleHit, _archived],
          onSelect: (_) {},
        ),
      ),
    );

    expect(find.byKey(const Key('conversation_search_result_c1')), findsOne);
    expect(find.byKey(const Key('conversation_search_result_c3')), findsOne);
    final titles = tester
        .widgetList<HighlightedSegments>(find.byType(HighlightedSegments))
        .toList();
    expect(titles.first.segments, _titleHit.titleSegments);
    expect(find.text('Old notes', findRichText: true), findsOne);
  });

  testWidgets('shows at most three snippets per row', (tester) async {
    await tester.pumpWidget(
      _host(
        ConversationSearchResultsView(
          results: [_messageHits],
          onSelect: (_) {},
        ),
      ),
    );

    expect(find.textContaining('snippet 1', findRichText: true), findsOne);
    expect(find.textContaining('snippet 3', findRichText: true), findsOne);
    expect(find.textContaining('snippet 4', findRichText: true), findsNothing);
  });

  testWidgets('a title-only hit shows no snippets', (tester) async {
    await tester.pumpWidget(
      _host(
        ConversationSearchResultsView(results: [_titleHit], onSelect: (_) {}),
      ),
    );

    expect(find.byType(HighlightedSegments), findsOneWidget);
  });

  testWidgets('shows the archived badge only when archivedAt is set', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        ConversationSearchResultsView(
          results: [_titleHit, _archived],
          onSelect: (_) {},
        ),
      ),
    );

    expect(find.text('Archived'), findsOneWidget);
    expect(
      find.descendant(
        of: find.byKey(const Key('conversation_search_result_c3')),
        matching: find.text('Archived'),
      ),
      findsOneWidget,
    );
  });

  testWidgets('a tap fires onSelect with the conversation id', (tester) async {
    final selected = <String>[];
    await tester.pumpWidget(
      _host(
        ConversationSearchResultsView(
          results: [_titleHit, _messageHits],
          onSelect: selected.add,
        ),
      ),
    );

    await tester.tap(find.byKey(const Key('conversation_search_result_c2')));

    expect(selected, ['c2']);
  });

  testWidgets('an empty result list shows "No conversations match"', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(ConversationSearchResultsView(results: const [], onSelect: (_) {})),
    );

    expect(find.text('No conversations match'), findsOneWidget);
  });

  testWidgets('each row is a button for screen readers', (tester) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(
      _host(
        ConversationSearchResultsView(results: [_titleHit], onSelect: (_) {}),
      ),
    );

    expect(
      tester.getSemantics(
        find.byKey(const Key('conversation_search_result_c1')),
      ),
      matchesSemantics(
        isButton: true,
        isFocusable: true,
        hasTapAction: true,
        hasFocusAction: true,
        label: 'Lisbon trip',
      ),
    );
    handle.dispose();
  });
}
