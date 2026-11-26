@Tags(['golden'])
library;

import 'package:assistant_ui/assistant_ui.dart';
import 'package:assistant_ui/models.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:material_ui/material_ui.dart';

import '../support/golden.dart';

const _plain = false;
const _hit = true;

const _titleHit = ConversationSearchItem(
  id: 'c1',
  title: 'Lisbon trip',
  titleSegments: [
    SearchSegmentItem(text: 'Lisbon', highlighted: _hit),
    SearchSegmentItem(text: ' trip', highlighted: _plain),
  ],
);

const _messageHits = ConversationSearchItem(
  id: 'c2',
  title: 'Weekend plans',
  titleSegments: [
    SearchSegmentItem(text: 'Weekend plans', highlighted: _plain),
  ],
  matches: [
    [
      SearchSegmentItem(text: '…booked the ', highlighted: _plain),
      SearchSegmentItem(text: 'hotel', highlighted: _hit),
      SearchSegmentItem(text: ' near the station', highlighted: _plain),
    ],
    [
      SearchSegmentItem(text: 'Which ', highlighted: _plain),
      SearchSegmentItem(text: 'hotel', highlighted: _hit),
      SearchSegmentItem(text: ' has a late check-in?', highlighted: _plain),
    ],
  ],
);

final _archived = ConversationSearchItem(
  id: 'c3',
  title: 'Old hotel notes',
  titleSegments: const [
    SearchSegmentItem(text: 'Old ', highlighted: _plain),
    SearchSegmentItem(text: 'hotel', highlighted: _hit),
    SearchSegmentItem(text: ' notes', highlighted: _plain),
  ],
  archivedAt: DateTime.utc(2026, 1, 2),
);

const _longSnippet = ConversationSearchItem(
  id: 'c4',
  title: 'Migration runbook',
  titleSegments: [
    SearchSegmentItem(text: 'Migration runbook', highlighted: _plain),
  ],
  matches: [
    [
      SearchSegmentItem(
        text:
            '…walk through every step of the migration, including the '
            'rollback plan, the ',
        highlighted: _plain,
      ),
      SearchSegmentItem(text: 'monitoring', highlighted: _hit),
      SearchSegmentItem(
        text:
            ' dashboards to watch afterwards and who is on call for the '
            'first week…',
        highlighted: _plain,
      ),
    ],
  ],
);

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('highlighted_segments (${brightness.name})', (tester) async {
      await pumpGolden(
        tester,
        const Padding(
          padding: EdgeInsets.all(16),
          child: HighlightedSegments([
            SearchSegmentItem(text: 'Book the ', highlighted: _plain),
            SearchSegmentItem(text: 'hotel', highlighted: _hit),
            SearchSegmentItem(text: ' in ', highlighted: _plain),
            SearchSegmentItem(text: 'Lisbon', highlighted: _hit),
          ]),
        ),
        size: const Size(360, 60),
        brightness: brightness,
      );
      await tester.pump(const Duration(milliseconds: 50));

      await expectLater(
        find.byType(MaterialApp),
        matchesGoldenFile(
          'goldens/highlighted_segments_${brightness.name}.png',
        ),
      );
    });
  }

  final cases = <String, ({List<ConversationSearchItem> results, Size size})>{
    'conversation_search_title_hit': (
      results: const [_titleHit],
      size: const Size(360, 120),
    ),
    'conversation_search_message_hits': (
      results: const [_messageHits],
      size: const Size(360, 180),
    ),
    'conversation_search_archived': (
      results: [_archived],
      size: const Size(360, 120),
    ),
    'conversation_search_empty': (
      results: const [],
      size: const Size(360, 120),
    ),
    'conversation_search_long_snippet': (
      results: const [_longSnippet],
      size: const Size(360, 140),
    ),
  };

  for (final entry in cases.entries) {
    for (final brightness in Brightness.values) {
      testWidgets('${entry.key} (${brightness.name})', (tester) async {
        await pumpGolden(
          tester,
          ConversationSearchResultsView(
            results: entry.value.results,
            onSelect: (_) {},
          ),
          size: entry.value.size,
          brightness: brightness,
        );
        await tester.pump(const Duration(milliseconds: 50));

        await expectLater(
          find.byType(MaterialApp),
          matchesGoldenFile('goldens/${entry.key}_${brightness.name}.png'),
        );
      });
    }
  }
}
